const anchor = require("@project-serum/anchor");
const { assert } = require("chai");
const nativeAssert = require("assert");

describe("interface", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const counter = anchor.workspace.Counter;
  const counterAuth = anchor.workspace.CounterAuth;
  it("Is initialized!", async () => {
    await counter.state.rpc.new(counterAuth.programId);

    const stateAccount = await counter.state.fetch();
    assert.isTrue(stateAccount.count.eq(new anchor.BN(0)));
    assert.isTrue(stateAccount.authProgram.equals(counterAuth.programId));
  });

  it("Should fail to go from even to even", async () => {
    await nativeAssert.rejects(
      async () => {
        await counter.state.rpc.setCount(new anchor.BN(4), {
          accounts: {
            authProgram: counterAuth.programId,
          },
        });
      },
      (err) => {
        if (err.toString().split("custom program error: 0x3a98").length !== 2) {
          return false;
        }
        return true;
      }
    );
  });

  it("Should succeed to go from even to odd", async () => {
    await counter.state.rpc.setCount(new anchor.BN(3), {
      accounts: {
        authProgram: counterAuth.programId,
      },
    });
    const stateAccount = await counter.state.fetch();
    assert.isTrue(stateAccount.count.eq(new anchor.BN(3)));
  });
});
