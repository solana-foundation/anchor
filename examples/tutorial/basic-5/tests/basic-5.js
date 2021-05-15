const anchor = require("@project-serum/anchor");
const assert = require("assert");

describe("basic-5", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.Basic5;

  const mint = anchor.web3.Keypair.generate();

  // Setup. Not important for the point of the example.
  it("Sets up the test", async () => {
    // Create the mint account.
    await program.rpc.createMint({
      accounts: {
        mint: mint.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      instructions: [await program.account.mint.createInstruction(mint)],
      signers: [mint],
    });
  });

  it("Creates an associated token account", async () => {
    // #region test
    // Calculate the associated token address.
    const authority = program.provider.wallet.publicKey;
    const associatedToken = await program.account.token.associatedAddress(
      authority,
      mint.publicKey
    );

    // Execute the transaction to create the associated token account.
    await program.rpc.createToken({
      accounts: {
        token: associatedToken,
        authority,
        mint: mint.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
    });

    // Fetch the new associated account.
    const account = await program.account.token.associated(
      authority,
      mint.publicKey
    );
    // #endregion test

    // Check it was created correctly.
    assert.ok(account.amount === 0);
    assert.ok(account.authority.equals(authority));
    assert.ok(account.mint.equals(mint.publicKey));
  });
});
