const assert = require("assert");
const anchor = require("@project-serum/anchor");
const { Account, Transaction, TransactionInstruction } = anchor.web3;

// sleep to allow logs to come in
const sleep = (ms) =>
  new Promise((resolve) => {
    setTimeout(() => resolve(), ms);
  });

const withLogTest = async (callback, expectedLog) => {
  let logTestOk = false;
  const listener = anchor.getProvider().connection.onLogs(
    "all",
    (logs) => {
      if (logs.logs.some((logLine) => logLine === expectedLog)) {
        logTestOk = true;
      } else {
        console.log(logs);
      }
    },
    "recent"
  );
  try {
    await callback();
  } catch (err) {
    anchor.getProvider().connection.removeOnLogsListener(listener);
    throw err;
  }
  await sleep(3000);
  anchor.getProvider().connection.removeOnLogsListener(listener);
  assert.ok(logTestOk);
};

describe("errors", () => {
  // Configure the client to use the local cluster.
  const localProvider = anchor.Provider.local();
  localProvider.opts.skipPreflight = true;
  anchor.setProvider(localProvider);

  const program = anchor.workspace.Errors;

  it("Emits a Hello error", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.hello();
        assert.ok(false);
      } catch (err) {
        const errMsg =
          "This is an error message clients will automatically display";
        assert.equal(err.toString(), errMsg);
        assert.equal(err.msg, errMsg);
        assert.equal(err.code, 6000);
      }
    }, "Program log: AnchorError thrown in programs/errors/src/lib.rs:13. Error Code: Hello. Error Number: 6000. Error Message: This is an error message clients will automatically display.");
  });

  it("Emits a Hello error via require!", async () => {
    try {
      const tx = await program.rpc.testRequire();
      assert.ok(false);
    } catch (err) {
      const errMsg =
        "This is an error message clients will automatically display";
      assert.equal(err.toString(), errMsg);
      assert.equal(err.msg, errMsg);
      assert.equal(err.code, 6000);
    }
  });

  it("Emits a Hello error via err!", async () => {
    try {
      const tx = await program.rpc.testErr();
      assert.ok(false);
    } catch (err) {
      const errMsg =
        "This is an error message clients will automatically display";
      assert.equal(err.toString(), errMsg);
      assert.equal(err.msg, errMsg);
      assert.equal(err.code, 6000);
    }
  });

  it("Logs a ProgramError", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.testProgramError();
        assert.ok(false);
      } catch (err) {
        // No-op (withLogTest expects the callback to catch the initial tx error)
      }
    }, "Program log: ProgramError occurred. Error Code: InvalidAccountData. Error Number: 17179869184. Error Message: An account's data contents was invalid.");
  });

  it("Logs a ProgramError with source", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.testProgramErrorWithSource();
        assert.ok(false);
      } catch (err) {
        // No-op (withLogTest expects the callback to catch the initial tx error)
      }
    }, "Program log: ProgramError thrown in programs/errors/src/lib.rs:38. Error Code: InvalidAccountData. Error Number: 17179869184. Error Message: An account's data contents was invalid.");
  });

  it("Emits a HelloNoMsg error", async () => {
    try {
      const tx = await program.rpc.helloNoMsg();
      assert.ok(false);
    } catch (err) {
      const errMsg = "HelloNoMsg";
      assert.equal(err.toString(), errMsg);
      assert.equal(err.msg, errMsg);
      assert.equal(err.code, 6000 + 123);
    }
  });

  it("Emits a HelloNext error", async () => {
    try {
      const tx = await program.rpc.helloNext();
      assert.ok(false);
    } catch (err) {
      const errMsg = "HelloNext";
      assert.equal(err.toString(), errMsg);
      assert.equal(err.msg, errMsg);
      assert.equal(err.code, 6000 + 124);
    }
  });

  it("Emits a mut error", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.mutError({
          accounts: {
            myAccount: anchor.web3.SYSVAR_RENT_PUBKEY,
          },
        });
        assert.ok(false);
      } catch (err) {
        const errMsg = "A mut constraint was violated";
        assert.equal(err.toString(), errMsg);
        assert.equal(err.msg, errMsg);
        assert.equal(err.code, 2000);
      }
    }, "Program log: AnchorError caused by account: my_account. Error Code: ConstraintMut. Error Number: 2000. Error Message: A mut constraint was violated.");
  });

  it("Emits a has one error", async () => {
    try {
      const account = new Account();
      const tx = await program.rpc.hasOneError({
        accounts: {
          myAccount: account.publicKey,
          owner: anchor.web3.SYSVAR_RENT_PUBKEY,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        instructions: [
          await program.account.hasOneAccount.createInstruction(account),
        ],
        signers: [account],
      });
      assert.ok(false);
    } catch (err) {
      const errMsg = "A has_one constraint was violated";
      assert.equal(err.toString(), errMsg);
      assert.equal(err.msg, errMsg);
      assert.equal(err.code, 2001);
    }
  });

  // This test uses a raw transaction and provider instead of a program
  // instance since the client won't allow one to send a transaction
  // with an invalid signer account.
  it("Emits a signer error", async () => {
    let signature;
    const listener = anchor
      .getProvider()
      .connection.onLogs("all", (logs) => (signature = logs.signature));
    try {
      const tx = new Transaction();
      tx.add(
        new TransactionInstruction({
          keys: [
            {
              pubkey: anchor.web3.SYSVAR_RENT_PUBKEY,
              isWritable: false,
              isSigner: false,
            },
          ],
          programId: program.programId,
          data: program.coder.instruction.encode("signer_error", {}),
        })
      );
      await program.provider.send(tx);
      assert.ok(false);
    } catch (err) {
      await sleep(3000);
      anchor.getProvider().connection.removeOnLogsListener(listener);
      const errMsg = `Error: Raw transaction ${signature} failed ({"err":{"InstructionError":[0,{"Custom":3010}]}})`;
      assert.equal(err.toString(), errMsg);
    } finally {
      anchor.getProvider().connection.removeOnLogsListener(listener);
    }
  });

  it("Emits a raw custom error", async () => {
    try {
      const tx = await program.rpc.rawCustomError({
        accounts: {
          myAccount: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
      });
      assert.ok(false);
    } catch (err) {
      const errMsg = "HelloCustom";
      assert.equal(err.toString(), errMsg);
      assert.equal(err.msg, errMsg);
      assert.equal(err.code, 6000 + 125);
    }
  });

  it("Emits a account not initialized error", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.accountNotInitializedError({
          accounts: {
            notInitializedAccount: new anchor.web3.Keypair().publicKey,
          },
        });
        assert.fail(
          "Unexpected success in creating a transaction that should have fail with `AccountNotInitialized` error"
        );
      } catch (err) {
        const errMsg =
          "The program expected this account to be already initialized";
        assert.equal(err.toString(), errMsg);
      }
    }, "Program log: AnchorError caused by account: not_initialized_account. Error Code: AccountNotInitialized. Error Number: 3012. Error Message: The program expected this account to be already initialized.");
  });
});
