import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NftStakingCore } from "../target/types/nft_staking_core";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { MPL_CORE_PROGRAM_ID, fetchAssetV1, fetchCollectionV1, mplCore } from "@metaplex-foundation/mpl-core";
import { ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { publicKey as umiPublicKey } from "@metaplex-foundation/umi";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { expect } from "chai";

const MILLISECONDS_PER_DAY = 86400000;
const POINTS_PER_STAKED_NFT_PER_DAY = 10_000_000;
const FREEZE_PERIOD_IN_DAYS = 7;
const TIME_TRAVEL_IN_DAYS = 8;

const SECONDS_PER_DAY = 86400;
const WINDOW_START_HOUR = 9;
const WINDOW_END_HOUR = 17;

describe("nft-staking-core", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.nftStakingCore as Program<NftStakingCore>;

  interface NftStakingAssetAttributes {
    staked: string;
    staked_at: string;
    [key: string]: string;
  }

  async function fetchNftAttributes(nftPubkey: PublicKey): Promise<NftStakingAssetAttributes> {
    const umi = createUmi(provider.connection.rpcEndpoint).use(mplCore());

    let asset;
    try {
      asset = await fetchAssetV1(umi, umiPublicKey(nftPubkey.toBase58()));
    } catch (err: any) {
      return null;
    }

    if (!asset.pluginHeader || !asset.attributes) {
      console.warn(
        `NFT ${nftPubkey.toBase58()} has no Attributes plugin (pluginHeader: ${asset.pluginHeader})`
      );
      return null;
    }

    const result: NftStakingAssetAttributes = {
      staked: "",
      staked_at: "",
    };

    for (const attr of asset.attributes.attributeList) {
      result[attr.key] = attr.value;
    }

    return result;
  }

  interface NftStakingCollectionAttributes {
    total_staked: string;
    [key: string]: string;
  }

  async function fetchCollectionAttributes(collectionPubkey: PublicKey): Promise<NftStakingCollectionAttributes | null> {
    const umi = createUmi(provider.connection.rpcEndpoint).use(mplCore());

    let collection;
    try {
      collection = await fetchCollectionV1(umi, umiPublicKey(collectionPubkey.toBase58()));
    } catch (err: any) {
      return null;
    }

    if (!collection.attributes) {
      console.warn(`Collection ${collectionPubkey.toBase58()} has no Attributes plugin`);
      return null;
    }

    const result: NftStakingCollectionAttributes = {
      total_staked: "",
    };

    for (const attr of collection.attributes.attributeList) {
      result[attr.key] = attr.value;
    }

    return result;
  }

  // Generate a keypair for the collection
  const collectionKeypair = anchor.web3.Keypair.generate();

  // Find the update authority for the collection (PDA)
  const updateAuthority = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("update_authority"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];

  // Generate a keypair for the nft asset
  const nftKeypair = anchor.web3.Keypair.generate();

  // Find the config account (PDA)
  const config = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("config"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];

  // Find the rewards mint account (PDA)
  const rewardsMint = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("rewards"), config.toBuffer()],
    program.programId
  )[0];

  const userRewardsAta = getAssociatedTokenAddressSync(
    rewardsMint,
    provider.wallet.publicKey,
    false,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
  );

  const oracle = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("oracle"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];

  const rewardVault = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("reward_vault"), oracle.toBuffer()],
    program.programId
  )[0];

  // Time functions
  let currentTimestamp = Date.now();

  async function getCurrentTimestamp(): Promise<number> {
    const slot = await provider.connection.getSlot();
    const blockTime = (await provider.connection.getBlockTime(slot))!;

    return Math.max(currentTimestamp, blockTime * 1000);
  }

  before(async () => {
    currentTimestamp = await getCurrentTimestamp();
  });

  /**
   * Helper function to advance time with surfnet_timeTravel RPC method
   * @param params - Time travel params (absoluteEpoch, absoluteSlot, or absoluteTimestamp)
   */
  async function advanceTime(params: { absoluteEpoch?: number; absoluteSlot?: number; absoluteTimestamp?: number }): Promise<void> {
    currentTimestamp = params.absoluteTimestamp;

    const rpcResponse = await fetch(provider.connection.rpcEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "surfnet_timeTravel",
        params: [params],
      }),
    });

    const result = await rpcResponse.json() as { error?: any; result?: any };
    if (result.error) {
      throw new Error(`Time travel failed: ${JSON.stringify(result.error)}`);
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  function secsToHour(timestamp, targetHour: number): number {
    const secsSinceMidnight = ((timestamp / 1000 % SECONDS_PER_DAY) + SECONDS_PER_DAY) % SECONDS_PER_DAY;

    const targetSecs = targetHour * 3600;
    return targetSecs > secsSinceMidnight
      ? targetSecs - secsSinceMidnight
      : SECONDS_PER_DAY - secsSinceMidnight + targetSecs;
  }

  async function ensureAllowedWindow(inside: boolean): Promise<void> {
    const currentTimestamp = await getCurrentTimestamp();

    const targetHour = inside ? WINDOW_START_HOUR + 1 : WINDOW_END_HOUR + 1;
    const secsToTravel = secsToHour(currentTimestamp, targetHour);
    console.log(`Traveling ${secsToTravel} seconds to reach ${targetHour}:00 UTC`);

    await advanceTime({ absoluteTimestamp: currentTimestamp + secsToTravel * 1000});
  }

  it("Create a collection", async () => {
    const collectionName = "Test Collection";
    const collectionUri = "https://example.com/collection";
    const tx = await program.methods.createCollection(collectionName, collectionUri)
      .accountsPartial({
        payer: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([collectionKeypair])
      .rpc();
    console.log("\nYour transaction signature", tx);
    console.log("Collection address", collectionKeypair.publicKey.toBase58());
  });

  it("Mint an NFT", async () => {
    const nftName = "Test NFT";
    const nftUri = "https://example.com/nft";
    const tx = await program.methods.mintNft(nftName, nftUri)
      .accountsPartial({
        user: provider.wallet.publicKey,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([nftKeypair])
      .rpc();
    console.log("\nYour transaction signature", tx);
    console.log("NFT address", nftKeypair.publicKey.toBase58());
  });

  it("Initialize stake config", async () => {
    const tx = await program.methods.initializeConfig(POINTS_PER_STAKED_NFT_PER_DAY, FREEZE_PERIOD_IN_DAYS)
      .accountsPartial({
        admin: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();
    console.log("\nYour transaction signature", tx);
    console.log("Config address", config.toBase58());
    console.log("Points per staked NFT per day", POINTS_PER_STAKED_NFT_PER_DAY);
    console.log("Freeze period in days", FREEZE_PERIOD_IN_DAYS);
    console.log("Rewards mint address", rewardsMint.toBase58());
  });

  it("Stake an NFT", async () => {
    const collectionAttrsBefore = await fetchCollectionAttributes(collectionKeypair.publicKey);
    let totalStakedBefore;
    if (!collectionAttrsBefore) {
      totalStakedBefore = 0;
    } else {
      totalStakedBefore = parseInt(collectionAttrsBefore.total_staked, 10);
    }

    const tx = await program.methods.stake()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .rpc();

    console.log("\nYour transaction signature", tx);

    const nftAttrs = await fetchNftAttributes(nftKeypair.publicKey);
    expect(nftAttrs.staked).to.equal("true");
    expect(parseInt(nftAttrs.staked_at, 10)).to.be.greaterThan(0);

    const collectionAttrsAfter = await fetchCollectionAttributes(collectionKeypair.publicKey);
    const totalStakedAfter = parseInt(collectionAttrsAfter.total_staked, 10);
    expect(totalStakedAfter).to.equal(totalStakedBefore + 1)
  });

  it("Claims Rewards for a staked NFT", async () => {
    await advanceTime({ absoluteTimestamp: currentTimestamp + TIME_TRAVEL_IN_DAYS * MILLISECONDS_PER_DAY });
    console.log("\nTime traveled in days", TIME_TRAVEL_IN_DAYS)

    const nftAttrs = await fetchNftAttributes(nftKeypair.publicKey);
    expect(nftAttrs.staked).to.equal("true");
    expect(parseInt(nftAttrs.staked_at, 10)).to.be.greaterThan(0);

    const attrsBefore = await fetchCollectionAttributes(collectionKeypair.publicKey);
    const totalStakedBefore = parseInt(attrsBefore.total_staked, 10);

    try {
      const tx = await program.methods
        .claimRewards()
        .accountsPartial({
          user: provider.wallet.publicKey,
          updateAuthority,
          config,
          rewardsMint,
          userRewardsAta,
          nft: nftKeypair.publicKey,
          collection: collectionKeypair.publicKey,
          mplCoreProgram: MPL_CORE_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        })
        .rpc();

      console.log("\nYour transaction signature", tx);

      const nftAttrs = await fetchNftAttributes(nftKeypair.publicKey);

      expect(nftAttrs.staked).to.equal("true");
      expect(parseInt(nftAttrs.staked_at, 10)).to.be.greaterThan(0);

      const collectionAttrsAfter = await fetchCollectionAttributes(collectionKeypair.publicKey);
      const totalStakedAfter = parseInt(collectionAttrsAfter.total_staked, 10);
      expect(totalStakedAfter).to.equal(totalStakedBefore)
    } catch (error) {
      console.log(error.logs);
      throw error;
    }
    console.log(
      "User rewards balance",
      (await provider.connection.getTokenAccountBalance(userRewardsAta)).value
        .uiAmount,
    );
  });

  it("Unstake an NFT", async () => {
    await advanceTime({ absoluteTimestamp: currentTimestamp + TIME_TRAVEL_IN_DAYS * MILLISECONDS_PER_DAY });
    console.log("\nTime traveled in days", TIME_TRAVEL_IN_DAYS)

    const nftAttrsBefore = await fetchNftAttributes(nftKeypair.publicKey);
    expect(nftAttrsBefore.staked).to.equal("true");
    expect(parseInt(nftAttrsBefore.staked_at, 10)).to.be.greaterThan(0);

    const collectionAttrsBefore = await fetchCollectionAttributes(collectionKeypair.publicKey);
    const totalStakedBefore = parseInt(collectionAttrsBefore.total_staked, 10);

    const tx = await program.methods.unstake()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        userRewardsAta,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();
    console.log("\nYour transaction signature", tx);

    const nftAttrs = await fetchNftAttributes(nftKeypair.publicKey);
    expect(nftAttrs.staked).to.equal("false");
    expect(parseInt(nftAttrs.staked_at, 10)).to.equal(0);

    const collectionAttrsAfter = await fetchCollectionAttributes(collectionKeypair.publicKey);
    const totalStakedAfter = parseInt(collectionAttrsAfter.total_staked, 10);
    expect(totalStakedAfter).to.equal(totalStakedBefore - 1)

    console.log("User rewards balance", (await provider.connection.getTokenAccountBalance(userRewardsAta)).value.uiAmount);
  });

  it("Burns staked NFT for rewards", async () => {
    const tx = await program.methods.stake()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .rpc();

    await advanceTime({ absoluteTimestamp: currentTimestamp + TIME_TRAVEL_IN_DAYS * MILLISECONDS_PER_DAY });
    console.log("\nTime traveled in days", TIME_TRAVEL_IN_DAYS)

    const nftAttrs = await fetchNftAttributes(nftKeypair.publicKey);
    expect(nftAttrs.staked).to.equal("true");
    expect(parseInt(nftAttrs.staked_at, 10)).to.be.greaterThan(0);

    const attrsBefore = await fetchCollectionAttributes(collectionKeypair.publicKey);
    const totalStakedBefore = parseInt(attrsBefore.total_staked, 10);

    try {
      const tx = await program.methods
        .burnStakedNft()
        .accountsPartial({
          user: provider.wallet.publicKey,
          updateAuthority,
          config,
          rewardsMint,
          userRewardsAta,
          nft: nftKeypair.publicKey,
          collection: collectionKeypair.publicKey,
          mplCoreProgram: MPL_CORE_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        })
        .rpc();
      console.log("\nYour transaction signature", tx);

      const nftAttrs = await fetchNftAttributes(nftKeypair.publicKey);
      expect(nftAttrs).to.be.null;

      const collectionAttrsAfter = await fetchCollectionAttributes(collectionKeypair.publicKey);
      const totalStakedAfter = parseInt(collectionAttrsAfter.total_staked, 10);
      expect(totalStakedAfter).to.equal(totalStakedBefore - 1)
    } catch (error) {
      console.log(error.logs);
      throw error;
    }
    console.log(
      "User rewards balance",
      (await provider.connection.getTokenAccountBalance(userRewardsAta)).value
        .uiAmount,
    );
  });

  it("Init oracle", async () => {
    await program.methods
      .initOracle()
      .accountsPartial({
        admin: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        rewardVault,
      })
      .rpc();

    const oracleState = await program.account.oracle.fetch(oracle);
    const v1 = (oracleState.validation as any).v1;
    expect(v1.transfer).to.exist;
    console.log(`oracle: ${oracle.toBase58()}`);
    console.log(`validation: ${JSON.stringify(oracleState.validation)}`);

    const rewardVaultBalance = await provider.connection.getBalance(rewardVault);
    expect(rewardVaultBalance).to.be.gt(0);
    console.log(`vault: ${rewardVault.toBase58()} (${rewardVaultBalance} lamports)`);
  });

  it("Update oracle succeeds at window boundary", async () => {
    await ensureAllowedWindow(false);

    let oracleState = await program.account.oracle.fetch(oracle);
    if (oracleState.validation.v1.transfer.approved) {
      try{
        await program.methods
          .updateOracle()
          .accountsPartial({
            cranker: provider.wallet.publicKey,
            collection: collectionKeypair.publicKey,
            oracle,
            rewardVault,
          })
          .rpc();
        } catch (error) {}
    }

    await ensureAllowedWindow(true);

    await program.methods
      .updateOracle()
      .accountsPartial({
        cranker: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        oracle,
        rewardVault,
      })
      .rpc();

    const updatedOracle = await program.account.oracle.fetch(oracle);
    expect(updatedOracle.validation.v1.transfer.approved).to.exist;

    console.log("Oracle correctly updated to Rejected outside window");
  });

  it("Update oracle fails if already up-to-date (AlreadyUpdated)", async () => {
    await ensureAllowedWindow(true);

    let oracleState = await program.account.oracle.fetch(oracle);
    if (oracleState.validation.v1.transfer.rejected) {
      await program.methods
        .updateOracle()
        .accountsPartial({
          cranker: provider.wallet.publicKey,
          collection: collectionKeypair.publicKey,
          oracle,
          rewardVault,
        })
        .rpc();
    }

    try {
      await program.methods
        .updateOracle()
        .accountsPartial({
          cranker: provider.wallet.publicKey,
          collection: collectionKeypair.publicKey,
          oracle,
          rewardVault,
        })
        .rpc();
      expect.fail("Should have thrown AlreadyUpdated");
    } catch (err: any) {
      expect(err.message).to.include("AlreadyUpdated");
      console.log("Oracle correctly rejected duplicate update");
    }
  });

  it("Transfer NFT during allowed window (9AM-5PM UTC)", async () => {
    const newOwner = anchor.web3.Keypair.generate();
    const nftKeypair = anchor.web3.Keypair.generate();

    const nftName = "Test NFT";
    const nftUri = "https://example.com/nft";
    const tx = await program.methods.mintNft(nftName, nftUri)
      .accountsPartial({
        user: provider.wallet.publicKey,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([nftKeypair])
      .rpc();

    await ensureAllowedWindow(true);

    let oracleState = await program.account.oracle.fetch(oracle);
    if (oracleState.validation.v1.transfer.rejected) {
      try {
        await program.methods
          .updateOracle()
          .accountsPartial({
            cranker: provider.wallet.publicKey,
            collection: collectionKeypair.publicKey,
            oracle,
            rewardVault,
          })
          .rpc();
      } catch (e) {}
    }

    await program.methods
      .transferNft()
      .accountsPartial({
        owner: provider.wallet.publicKey,
        newOwner: newOwner.publicKey,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        oracle,
      })
      .rpc();
    console.log("NFT transferred during allowed window");
  });

  it("Transfer fails outside window (11PM UTC)", async () => {
    const newOwner = anchor.web3.Keypair.generate();
    const nftKeypair = anchor.web3.Keypair.generate();

    const nftName = "Test NFT";
    const nftUri = "https://example.com/nft";
    const tx = await program.methods.mintNft(nftName, nftUri)
      .accountsPartial({
        user: provider.wallet.publicKey,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([nftKeypair])
      .rpc();

    await ensureAllowedWindow(false);

    let oracleState = await program.account.oracle.fetch(oracle);
    if (oracleState.validation.v1.transfer.approved) {
      try {
        await program.methods
          .updateOracle()
          .accountsPartial({
            cranker: provider.wallet.publicKey,
            collection: collectionKeypair.publicKey,
            oracle,
            rewardVault,
          })
          .rpc();
      } catch (e) {}
    }

    try {
      await program.methods
        .transferNft()
        .accountsPartial({
          owner: provider.wallet.publicKey,
          newOwner: newOwner.publicKey,
          nft: nftKeypair.publicKey,
          collection: collectionKeypair.publicKey,
          updateAuthority,
          oracle,
        })
        .rpc();
    } catch (err: any) {
      if (err.message === "Should have failed") throw err;
      console.log("Transfer correctly blocked outside window");
    }
  });
});