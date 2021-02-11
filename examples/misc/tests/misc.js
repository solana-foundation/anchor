const anchor = require('@project-serum/anchor');
const assert = require("assert");

describe("misc", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  it("Can use u128 and i128", async () => {
    const data = new anchor.web3.Account();
    const program = anchor.workspace.Misc;
    const tx = await program.rpc.initialize(
      new anchor.BN(1234),
      new anchor.BN(22),
      {
        accounts: {
          data: data.publicKey,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        signers: [data],
        instructions: [await program.account.data.createInstruction(data)],
      }
    );
    const dataAccount = await program.account.data(data.publicKey);
    assert.ok(dataAccount.udata.eq(new anchor.BN(1234)));
    assert.ok(dataAccount.idata.eq(new anchor.BN(22)));
  });
});
