import * as anchor from "@anchor-lang/core";
import assert from "assert";

describe("no-std-test", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.NoStdTest;

  it("Initializes a data account", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialData = new anchor.BN(42);

    await program.methods
      .initialize(initialData)
      .accounts({
        dataAccount: dataAccount.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    const account = await program.account.dataAccount.fetch(dataAccount.publicKey);
    assert.ok(account.data.eq(initialData));
  });

  it("Updates a data account", async () => {
    const dataAccount = anchor.web3.Keypair.generate();
    const initialData = new anchor.BN(10);
    const newData = new anchor.BN(20);

    // Initialize first
    await program.methods
      .initialize(initialData)
      .accounts({
        dataAccount: dataAccount.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([dataAccount])
      .rpc();

    // Update
    await program.methods
      .update(newData)
      .accounts({
        dataAccount: dataAccount.publicKey,
      })
      .rpc();

    const account = await program.account.dataAccount.fetch(dataAccount.publicKey);
    assert.ok(account.data.eq(newData));
  });
});
