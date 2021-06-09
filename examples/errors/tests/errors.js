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
      assert.equal(err.code, 300);
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
      assert.equal(err.code, 300 + 123);
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
      assert.equal(err.code, 300 + 124);
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
      assert.equal(err.code, 140);
    }
  });

  it("Emits a belongs to error", async () => {
    try {
      const account = new Account();
      const tx = await program.rpc.belongsToError({
        accounts: {
          myAccount: account.publicKey,
          owner: anchor.web3.SYSVAR_RENT_PUBKEY,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        instructions: [
          await program.account.belongsToAccount.createInstruction(account),
        ],
        signers: [account],
      });
      assert.ok(false);
    } catch (err) {
      const errMsg = "A belongs_to constraint was violated";
      assert.equal(err.toString(), errMsg);
      assert.equal(err.msg, errMsg);
      assert.equal(err.code, 141);
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
        "Error: failed to send transaction: Transaction simulation failed: Error processing Instruction 0: custom program error: 0x8e";
      assert.equal(err.toString(), errMsg);
    }
  });
});
