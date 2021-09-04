const assert = require("assert");
const anchor = require("@project-serum/anchor");

describe("composite", () => {
  const provider = anchor.Provider.local();

  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  it("Is initialized!", async () => {
    const program = anchor.workspace.Composite;

    const dummyA = anchor.web3.Keypair.generate();
    const dummyB = anchor.web3.Keypair.generate();

    const tx = await program.rpc.initialize({
      accounts: {
        dummyA: dummyA.publicKey,
        dummyB: dummyB.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [dummyA, dummyB],
      instructions: [
        await program.account.dummyA.createInstruction(dummyA),
        await program.account.dummyB.createInstruction(dummyB),
      ],
    });

    await program.rpc.compositeUpdate(
      new anchor.BN(1234),
      new anchor.BN(4321),
      {
        accounts: {
          foo: {
            dummyA: dummyA.publicKey,
          },
          bar: {
            dummyB: dummyB.publicKey,
          },
        },
      }
    );

    const dummyAAccount = await program.account.dummyA.fetch(dummyA.publicKey);
    const dummyBAccount = await program.account.dummyB.fetch(dummyB.publicKey);

    assert.ok(dummyAAccount.data.eq(new anchor.BN(1234)));
    assert.ok(dummyBAccount.data.eq(new anchor.BN(4321)));
  });
});
