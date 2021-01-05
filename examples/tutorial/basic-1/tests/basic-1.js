const assert = require('assert');
const anchor = require('@project-serum/anchor');

describe('basic-1', () => {

  // Use a local provider.
  const provider = anchor.Provider.local()

  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  it('Creates and initializes an account in two different transactions', async () => {
    // The program owning the account to create.
    const program = anchor.workspace.Basic1;

    // The Account to create.
    const myAccount = new anchor.web3.Account();

    // Create account transaction.
    const tx = new anchor.web3.Transaction();
    tx.add(
      anchor.web3.SystemProgram.createAccount({
        fromPubkey: provider.wallet.publicKey,
        newAccountPubkey: myAccount.publicKey,
        space: 8,
        lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        programId: program.programId,
      }),
    );

    // Execute the transaction against the cluster.
    await provider.send(tx, [myAccount]);

    // Execute the RPC.
    // #region code-separated
    await program.rpc.initialize(new anchor.BN(1234), {
      accounts: {
        myAccount: myAccount.publicKey,
      },
    });
    // #endregion code-separated

    // Fetch the newly created account from the cluster.
    const account = await program.account.myAccount(myAccount.publicKey);

    // Check it's state was initialized.
    assert.ok(account.data.eq(new anchor.BN(1234)));
  });


  it('Creates and initializes an account in a single atomic transaction', async () => {
    // The program to execute.
    const program = anchor.workspace.Basic1;

    // #region code
    // The Account to create.
    const myAccount = new anchor.web3.Account();

    // Atomically create the new account and initialize it with the program.
    await program.rpc.initialize(new anchor.BN(1234), {
      accounts: {
        myAccount: myAccount.publicKey,
      },
      signers: [myAccount],
      instructions: [
        anchor.web3.SystemProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          newAccountPubkey: myAccount.publicKey,
          space: 8,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
          programId: program.programId,
        }),
      ],
    });

    // Fetch the newly created account from the cluster.
    const account = await program.account.myAccount(myAccount.publicKey);

    // Check it's state was initialized.
    assert.ok(account.data.eq(new anchor.BN(1234)));
    // #endregion code
  });
});
