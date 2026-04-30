import * as anchor from "@anchor-lang/core";
import { Basic5 } from "../target/types/basic_5";

describe("basic-5", () => {
  const provider = anchor.AnchorProvider.local();

  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  const program = anchor.workspace.Basic5 as anchor.Program<Basic5>;
  const user = provider.wallet.publicKey;

  let [actionState] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("action-state"), user.toBuffer()],
    program.programId
  );

  it("basic-5: Robot actions!", async () => {
    // Create instruction: set up the Solana accounts to be used
    const createInstruction = await program.methods
      .create()
      .accounts({
        // actionState,
         /* 
        A accounts whose seeds are fully declared in the IDL 
        (e.g. actionState has pda.seeds = [{ kind: "const", value: [...] }])
        client derives the address at call time, no need to pass it.
        */
        // user,

        /*
        accounts with `relations: ["actionState"]` (user) tell the client
       "resolve this account from actionState's data".  Combined with the
       fact that `user` is also the wallet signer, the client resolves it
       from the provider automatically.
       */

        // systemProgram: anchor.web3.SystemProgram.programId,

        /*
         Known system programs whose address is fixed in the IDL
          (e.g. systemProgram → "11111111111111111111111111111111",
         tokenProgram  → resolved from the token interface constraint) client fills these in automatically.

        */

      })
      .instruction();
    // Walk instruction: Invoke the Robot to walk
    const walkInstruction = await program.methods
      .walk()
      .accounts({
        actionState,
        // user,
        /*
        accounts with `relations: ["actionState"]` (user) tell the client
       "resolve this account from actionState's data".  Combined with the
       fact that `user` is also the wallet signer, the client resolves it
       from the provider automatically.
       */
      })
      .instruction();
    // Run instruction: Invoke the Robot to run
    const runInstruction = await program.methods
      .run()
      .accounts({
        actionState,
        // user,
        /*
        accounts with `relations: ["actionState"]` (user) tell the client
       "resolve this account from actionState's data".  Combined with the
       fact that `user` is also the wallet signer, the client resolves it
       from the provider automatically.
       */
      })
      .instruction();
    // Jump instruction: Invoke the Robot to jump
    const jumpInstruction = await program.methods
      .jump()
      .accounts({
        actionState,
        // user,
        /*
        accounts with `relations: ["actionState"]` (user) tell the client
       "resolve this account from actionState's data".  Combined with the
       fact that `user` is also the wallet signer, the client resolves it
       from the provider automatically.
       */
      })
      .instruction();
    // Reset instruction: Reset actions of the Robot
    const resetInstruction = await program.methods
      .reset()
      .accounts({
        actionState,
        // user,
        /*
        accounts with `relations: ["actionState"]` (user) tell the client
       "resolve this account from actionState's data".  Combined with the
       fact that `user` is also the wallet signer, the client resolves it
       from the provider automatically.
       */
      })
      .instruction();

    // Array of instructions
    const instructions: anchor.web3.TransactionInstruction[] = [
      createInstruction,
      walkInstruction,
      runInstruction,
      jumpInstruction,
      resetInstruction,
    ];

    await createAndSendV0Tx(instructions);
  });

  async function createAndSendV0Tx(
    txInstructions: anchor.web3.TransactionInstruction[]
  ) {
    // Step 1 - Fetch the latest blockhash
    let latestBlockhash = await provider.connection.getLatestBlockhash(
      "confirmed"
    );
    console.log(
      "   :white_check_mark: - Fetched latest blockhash. Last Valid Height:",
      latestBlockhash.lastValidBlockHeight
    );

    // Step 2 - Generate Transaction Message
    const messageV0 = new anchor.web3.TransactionMessage({
      payerKey: user,
      recentBlockhash: latestBlockhash.blockhash,
      instructions: txInstructions,
    }).compileToV0Message();
    console.log("   :white_check_mark: - Compiled Transaction Message");
    const transaction = new anchor.web3.VersionedTransaction(messageV0);

    // Step 3 - Sign your transaction with the required `Signers`
    provider.wallet.signTransaction(transaction);
    console.log("   :white_check_mark: - Transaction Signed");

    // Step 4 - Send our v0 transaction to the cluster
    const txid = await provider.connection.sendTransaction(transaction, {
      maxRetries: 5,
    });
    console.log("   :white_check_mark: - Transaction sent to network");

    // Step 5 - Confirm Transaction
    const confirmation = await provider.connection.confirmTransaction({
      signature: txid,
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
    });
    if (confirmation.value.err) {
      throw new Error(
        `   :x: - Transaction not confirmed.\nReason: ${confirmation.value.err}`
      );
    }

    console.log(":tada: Transaction Successfully Confirmed!");
    let result = await program.account.actionState.fetch(actionState);
    console.log("Robot action state details: ", result);
  }
});