import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TuktukGptOracle } from "../target/types/tuktuk_gpt_oracle";
import { 
  init as initTuktuk, 
  taskQueueAuthorityKey, 
  PROGRAM_ID as TUKTUK_PROGRAM_ID 
} from "@helium/tuktuk-sdk";

describe("tuktuk-gpt-oracle", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const TASK_QUEUE = new anchor.web3.PublicKey("BNRQuLjUL35zktSKevk2Y4FKkqgYrUj9mrsWM8c59n8V");

  const ORACLE_PROGRAM_ID = new anchor.web3.PublicKey("LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab");

  const TEST_TEXT = "Hello. Tell me about Solana Alpenglow";

  const program = anchor.workspace.tuktukGptOracle as Program<TuktukGptOracle>;

  const wallet = provider.wallet as anchor.Wallet;

  const queueAuthority = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("queue_authority")], 
    program.programId
  )[0];
  const taskQueueAuthority = taskQueueAuthorityKey(TASK_QUEUE, queueAuthority)[0];

  console.log("taskQueue", TASK_QUEUE)
  console.log("queueAuthority", queueAuthority)
  console.log("taskQueueAuthority", taskQueueAuthority)

  const counterPda = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("counter")],
    ORACLE_PROGRAM_ID
  )[0];

  const agentPda = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("agent"),
    wallet.publicKey.toBuffer()],
    program.programId
  )[0];

  function getLlmContextPda(count: number) {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("test-context"),
      new Uint8Array(new Uint32Array([count]).buffer)
      ],
      ORACLE_PROGRAM_ID
    );
  }

  function getInteractionPda(context: anchor.web3.PublicKey) {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("interaction"),
      wallet.publicKey.toBuffer(),
      context.toBuffer()
      ],
      ORACLE_PROGRAM_ID
    )[0];
  }

  it("Initialize", async () => {
    const agentInfo = await provider.connection.getAccountInfo(agentPda);

    if (agentInfo) {
      console.log("Already initialized");
      return
    }

    const counterInfo = await provider.connection.getAccountInfo(counterPda);
    const count = counterInfo!.data.readUInt32LE(8);

    const [llmContextPda] = getLlmContextPda(count);

    const tx = await program.methods.initialize()
      .accountsPartial({
        payer: wallet.publicKey,
        agent: agentPda,
        counter: counterPda,
        llmContext: llmContextPda,
        oracleProgram: ORACLE_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("Initialize tx:", tx);
  });

  it("Interaction", async () => {
    const agentAccount = await program.account.agent.fetch(agentPda);
    const llmContextPda = agentAccount.context;
    const interactionPda = getInteractionPda(llmContextPda);

    const tx = await program.methods.interactAgent(TEST_TEXT)
      .accountsPartial({
        payer: wallet.publicKey,
        interaction: interactionPda,
        contextAccount: llmContextPda,
      }).rpc();

    console.log("Interaction tx:", tx);
  });

  it("Schedule", async () => {
    const tuktukProgram = await initTuktuk(provider);

    const agentAccount = await program.account.agent.fetch(agentPda);

    const llmContextPda = agentAccount.context;
    const interactionPda = getInteractionPda(llmContextPda);

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

    const [tqAuthorityPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("task_queue_authority"),
        TASK_QUEUE.toBuffer(),
        queueAuthority.toBuffer(),
      ],
      TUKTUK_PROGRAM_ID
    );

      console.log("taskId:", taskId);
      console.log("task:", taskAccount.toBase58());

      const tx = await program.methods
        .schedule(TEST_TEXT, taskId)
        .accountsPartial({
          payer: wallet.publicKey,
          interaction: interactionPda,
          agent: agentPda,
          contextAccount: llmContextPda,
          taskQueue: TASK_QUEUE,
          taskQueueAuthority: tqAuthorityPda,
          task: taskAccount,
          queueAuthority,
        })
        .rpc({ skipPreflight: true, commitment: "confirmed" });

    console.log("Schedule tx:", tx);
  });
});