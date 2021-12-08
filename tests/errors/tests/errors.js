const assert = require("assert");
const anchor = require('@project-serum/anchor');
const { Account, Transaction, TransactionInstruction } = anchor.web3;

describe("errors", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.local());

  const program = anchor.workspace.Errors;

  it("Emits a Hello error", async () => {
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
    try {
      const account = new Account();
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
      const errMsg =
        "Error: failed to send transaction: Transaction simulation failed: Error processing Instruction 0: custom program error: 0x7d2";
      assert.equal(err.toString(), errMsg);
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
    try {
      const tx = await program.rpc.accountNotInitializedError({
        accounts: {
          notInitializedAccount: (new anchor.web3.Keypair()).publicKey
        },
      });
      assert.fail("Unexpected success in creating a transaction that should have fail with `AccountNotInitialized` error");
    } catch (err) {
      const errMsg = "The program expected this account to be already initialized";
      assert.equal(err.toString(), errMsg);
    }
  });
});
