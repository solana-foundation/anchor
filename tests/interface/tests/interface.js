const anchor = require('@project-serum/anchor');
const assert = require("assert");

describe("interface", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const counter = anchor.workspace.Counter;
  const counterAuth = anchor.workspace.CounterAuth;
  it("Is initialized!", async () => {
    await counter.state.rpc.new(counterAuth.programId);

    const stateAccount = await counter.state.fetch();
    assert.ok(stateAccount.count.eq(new anchor.BN(0)));
    assert.ok(stateAccount.authProgram.equals(counterAuth.programId));
  });

  it("Should fail to go from even to even", async () => {
    await assert.rejects(
      async () => {
        await counter.state.rpc.setCount(new anchor.BN(4), {
          accounts: {
            authProgram: counterAuth.programId,
          },
        });
      },
      (err) => {
        if (err.toString().split("custom program error: 0x32").length !== 2) {
          return false;
        }
        return true;
      }
    );
  });

  it("Shold succeed to go from even to odd", async () => {
    await counter.state.rpc.setCount(new anchor.BN(3), {
      accounts: {
        authProgram: counterAuth.programId,
      },
    });
    const stateAccount = await counter.state.fetch();
    assert.ok(stateAccount.count.eq(new anchor.BN(3)));
  });
});
