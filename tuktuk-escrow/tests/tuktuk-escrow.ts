import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TuktukEscrow } from "../target/types/tuktuk_escrow";
import { Keypair, PublicKey } from "@solana/web3.js";
import { 
  init as initTuktuk, 
  taskKey, 
  taskQueueAuthorityKey, 
  PROGRAM_ID as TUKTUK_PROGRAM_ID 
} from "@helium/tuktuk-sdk";
import { createMint, getAssociatedTokenAddressSync, getOrCreateAssociatedTokenAccount, mintTo, TOKEN_PROGRAM_ID } from "@solana/spl-token";

describe("tuktuk-escrow", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.tuktukEscrow as Program<TuktukEscrow>;
  const connection = provider.connection;
  const wallet = provider.wallet as anchor.Wallet;
  const payer = wallet.payer;

  let mintA: PublicKey;
  let mintB: PublicKey;
  let makerAtaA: any;
  let escrowPda: PublicKey;
  let vault: PublicKey;

  const seed = new anchor.BN(Math.floor(Math.random() * 1000000));
  const seedBuf = Buffer.alloc(8);
  seedBuf.writeBigUInt64LE(BigInt(seed.toString()));

  const TASK_QUEUE = new anchor.web3.PublicKey("BNRQuLjUL35zktSKevk2Y4FKkqgYrUj9mrsWM8c59n8V");

  const queueAuthority = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("queue_authority")], 
    program.programId
  )[0];
  const taskQueueAuthority = taskQueueAuthorityKey(TASK_QUEUE, queueAuthority)[0];

  before(async () => {
    mintA = await createMint(connection, payer, payer.publicKey, null, 6);
    console.log("mintA: ", mintA);
    mintB = await createMint(connection, payer, payer.publicKey, null, 6);
    [escrowPda] = PublicKey.findProgramAddressSync([Buffer.from("escrow"), payer.publicKey.toBuffer(), seedBuf], program.programId);
    vault = getAssociatedTokenAddressSync(mintA, escrowPda, true);

    const makerAtaAAddress = getAssociatedTokenAddressSync(mintA, payer.publicKey);

    // Create and fund it
    await getOrCreateAssociatedTokenAccount(
      connection, payer, mintA, payer.publicKey
    );

    await mintTo(
      connection, payer, mintA, makerAtaAAddress,
      payer.publicKey, 1_000_000_000
    );

    makerAtaA = makerAtaAAddress;
  });

  it("Make", async () => {
    const receive = new anchor.BN(1_0000_000);
    const deposit = new anchor.BN(1_000);

    console.log("escrow: ", escrowPda)
    console.log("seed: ", seed.toNumber())

    const tx = await program.methods.make(seed, deposit, receive).accounts({
      maker: payer.publicKey,
      mintA,
      mintB,
      tokenProgram: TOKEN_PROGRAM_ID
    }).rpc()

    console.log("Make tx: ", tx);
  })

  it("Schedule auto refund", async () => {
    let tuktukProgram = await initTuktuk(provider);
    const taskQueueAuthorityInfo = await provider.connection.getAccountInfo(taskQueueAuthority);

    if (!taskQueueAuthorityInfo) {
      console.log("Registering queue authority...");

      const regTx = await tuktukProgram.methods
        .addQueueAuthorityV0()
        .accounts({
          payer: wallet.publicKey,
          queueAuthority,
          taskQueue: TASK_QUEUE,
        })
        .rpc();

      console.log("Registered:", regTx);
    } else {
      console.log("Queue authority already registered.");
    }

    // find free task id
    const tqRaw = (await tuktukProgram.account.taskQueueV0.fetch(TASK_QUEUE)) as any;
    let taskId = 0;
    for (let i = 0; i < tqRaw.taskBitmap.length; i++) {
      if (tqRaw.taskBitmap[i] !== 0xff) {
        for (let bit = 0; bit < 8; bit++) {
          if ((tqRaw.taskBitmap[i] & (1 << bit)) === 0) {
            taskId = i * 8 + bit;
            break;
          }
        }
        break;
      }
    }

    const taskIdBuf = Buffer.alloc(2);
    taskIdBuf.writeUInt16LE(taskId);

    const [taskAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("task"), TASK_QUEUE.toBuffer(), taskIdBuf],
      TUKTUK_PROGRAM_ID
    );

    try {
      const tx = await program.methods.schedule(taskId).accountsPartial({
        maker: payer.publicKey,
        mintA,
        escrow: escrowPda,
        vault,
        task: taskAccount,
        taskQueue: TASK_QUEUE,
        taskQueueAuthority: taskQueueAuthority,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).rpc({skipPreflight: true});

      console.log("Schedule tx:", tx);
    } catch (error) {
      console.log("\nFull error:", error);
      if (error.logs) {
        console.log("\nTransaction logs:");
        error.logs.forEach(log => console.log(log));
      }
      throw error;
    }
  });
});