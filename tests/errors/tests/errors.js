const assert = require("assert");
const anchor = require("@project-serum/anchor");
const { Account, Transaction, TransactionInstruction } = anchor.web3;
const { TOKEN_PROGRAM_ID, Token } = require("@solana/spl-token");

// sleep to allow logs to come in
const sleep = (ms) =>
  new Promise((resolve) => {
    setTimeout(() => resolve(), ms);
  });

const withLogTest = async (callback, expectedLogs) => {
  let logTestOk = false;
  const listener = anchor.getProvider().connection.onLogs(
    "all",
    (logs) => {
      const index = logs.logs.findIndex(
        (logLine) => logLine === expectedLogs[0]
      );
      if (index === -1) {
        console.log("Expected: ");
        console.log(expectedLogs);
        console.log("Actual: ");
        console.log(logs);
      } else {
        const actualLogs = logs.logs.slice(index, index + expectedLogs.length);
        for (let i = 0; i < expectedLogs.length; i++) {
          if (actualLogs[i] !== expectedLogs[i]) {
            console.log("Expected: ");
            console.log(expectedLogs);
            console.log("Actual: ");
            console.log(logs);
            return;
          }
        }
        logTestOk = true;
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
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:13. Error Code: Hello. Error Number: 6000. Error Message: This is an error message clients will automatically display.",
    ]);
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
    }, [
      "Program log: ProgramError occurred. Error Code: InvalidAccountData. Error Number: 17179869184. Error Message: An account's data contents was invalid.",
    ]);
  });

  it("Logs a ProgramError with source", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.testProgramErrorWithSource();
        assert.ok(false);
      } catch (err) {
        // No-op (withLogTest expects the callback to catch the initial tx error)
      }
    }, [
      "Program log: ProgramError thrown in programs/errors/src/lib.rs:38. Error Code: InvalidAccountData. Error Number: 17179869184. Error Message: An account's data contents was invalid.",
    ]);
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
    }, [
      "Program log: AnchorError caused by account: my_account. Error Code: ConstraintMut. Error Number: 2000. Error Message: A mut constraint was violated.",
    ]);
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
    }, [
      "Program log: AnchorError caused by account: not_initialized_account. Error Code: AccountNotInitialized. Error Number: 3012. Error Message: The program expected this account to be already initialized.",
    ]);
  });

  it("Emits an AccountOwnedByWrongProgram error", async () => {
    let client = await Token.createMint(
      program.provider.connection,
      program.provider.wallet.payer,
      program.provider.wallet.publicKey,
      program.provider.wallet.publicKey,
      9,
      TOKEN_PROGRAM_ID
    );

    await withLogTest(async () => {
      try {
        const tx = await program.rpc.accountOwnedByWrongProgramError({
          accounts: {
            wrongAccount: client.publicKey,
          },
        });
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `AccountOwnedByWrongProgram` error"
        );
      } catch (err) {
        const errMsg =
          "The given account is owned by a different program than expected";
        assert.equal(err.toString(), errMsg);
      }
    }, [
      "Program log: AnchorError caused by account: wrong_account. Error Code: AccountOwnedByWrongProgram. Error Number: 3007. Error Message: The given account is owned by a different program than expected.",
      "Program log: Left:",
      "Program log: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      "Program log: Right:",
      "Program log: Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
    ]);
  });

  it("Emits a ValueMismatch error via require_eq", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.requireEq();
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `ValueMismatch` error"
        );
      } catch (err) {
        assert.equal(err.code, 6126);
      }
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:68. Error Code: ValueMismatch. Error Number: 6126. Error Message: ValueMismatch.",
      "Program log: Left: 5241",
      "Program log: Right: 124124124",
    ]);
  });

  it("Emits a RequireEqViolated error via require_eq", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.requireEqDefaultError();
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `ValueMismatch` error"
        );
      } catch (err) {
        assert.equal(err.code, 2501);
      }
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:73. Error Code: RequireEqViolated. Error Number: 2501. Error Message: A require_eq expression was violated.",
      "Program log: Left: 5241",
      "Program log: Right: 124124124",
    ]);
  });

  it("Emits a ValueMatch error via require_neq", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.requireNeq();
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `ValueMatch` error"
        );
      } catch (err) {
        assert.equal(err.code, 6127);
      }
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:78. Error Code: ValueMatch. Error Number: 6127. Error Message: ValueMatch.",
      "Program log: Left: 500",
      "Program log: Right: 500",
    ]);
  });

  it("Emits a RequireNeqViolated error via require_neq", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.requireNeqDefaultError();
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `RequireNeqViolated` error"
        );
      } catch (err) {
        assert.equal(err.code, 2503);
      }
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:83. Error Code: RequireNeqViolated. Error Number: 2503. Error Message: A require_neq expression was violated.",
      "Program log: Left: 500",
      "Program log: Right: 500",
    ]);
  });

  it("Emits a ValueMismatch error via require_keys_eq", async () => {
    const someAccount = anchor.web3.Keypair.generate().publicKey;
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.requireKeysEq({
          accounts: {
            someAccount,
          },
        });
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `ValueMismatch` error"
        );
      } catch (err) {
        assert.equal(err.code, 6126);
      }
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:88. Error Code: ValueMismatch. Error Number: 6126. Error Message: ValueMismatch.",
      "Program log: Left:",
      `Program log: ${someAccount}`,
      "Program log: Right:",
      `Program log: ${program.programId}`,
    ]);
  });

  it("Emits a RequireKeysEqViolated error via require_keys_eq", async () => {
    const someAccount = anchor.web3.Keypair.generate().publicKey;
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.requireKeysEqDefaultError({
          accounts: {
            someAccount,
          },
        });
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `ValueMismatch` error"
        );
      } catch (err) {
        assert.equal(err.code, 2502);
      }
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:97. Error Code: RequireKeysEqViolated. Error Number: 2502. Error Message: A require_keys_eq expression was violated.",
      "Program log: Left:",
      `Program log: ${someAccount}`,
      "Program log: Right:",
      `Program log: ${program.programId}`,
    ]);
  });

  it("Emits a ValueMatch error via require_keys_neq", async () => {
    const someAccount = program.programId;
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.requireKeysNeq({
          accounts: {
            someAccount,
          },
        });
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `ValueMatch` error"
        );
      } catch (err) {
        assert.equal(err.code, 6127);
      }
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:102. Error Code: ValueMatch. Error Number: 6127. Error Message: ValueMatch.",
      "Program log: Left:",
      `Program log: ${someAccount}`,
      "Program log: Right:",
      `Program log: ${program.programId}`,
    ]);
  });

  it("Emits a RequireKeysNeqViolated error via require_keys_neq", async () => {
    const someAccount = program.programId;
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.requireKeysNeqDefaultError({
          accounts: {
            someAccount,
          },
        });
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `RequireKeysNeqViolated` error"
        );
      } catch (err) {
        assert.equal(err.code, 2504);
      }
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:111. Error Code: RequireKeysNeqViolated. Error Number: 2504. Error Message: A require_keys_neq expression was violated.",
      "Program log: Left:",
      `Program log: ${someAccount}`,
      "Program log: Right:",
      `Program log: ${program.programId}`,
    ]);
  });

  it("Emits a ValueLessOrEqual error via require_gt", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.requireGt();
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `ValueLessOrEqual` error"
        );
      } catch (err) {
        assert.equal(err.code, 6129);
      }
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:116. Error Code: ValueLessOrEqual. Error Number: 6129. Error Message: ValueLessOrEqual.",
      "Program log: Left: 5",
      "Program log: Right: 10",
    ]);
  });

  it("Emits a RequireGtViolated error via require_gt", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.requireGtDefaultError();
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `RequireGtViolated` error"
        );
      } catch (err) {
        assert.equal(err.code, 2505);
      }
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:121. Error Code: RequireGtViolated. Error Number: 2505. Error Message: A require_gt expression was violated.",
      "Program log: Left: 10",
      "Program log: Right: 10",
    ]);
  });

  it("Emits a ValueLess error via require_gte", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.requireGte();
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `ValueLess` error"
        );
      } catch (err) {
        assert.equal(err.code, 6128);
      }
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:126. Error Code: ValueLess. Error Number: 6128. Error Message: ValueLess.",
      "Program log: Left: 5",
      "Program log: Right: 10",
    ]);
  });

  it("Emits a RequireGteViolated error via require_gte", async () => {
    await withLogTest(async () => {
      try {
        const tx = await program.rpc.requireGteDefaultError();
        assert.fail(
          "Unexpected success in creating a transaction that should have failed with `RequireGteViolated` error"
        );
      } catch (err) {
        assert.equal(err.code, 2506);
      }
    }, [
      "Program log: AnchorError thrown in programs/errors/src/lib.rs:131. Error Code: RequireGteViolated. Error Number: 2506. Error Message: A require_gte expression was violated.",
      "Program log: Left: 5",
      "Program log: Right: 10",
    ]);
  });
});
