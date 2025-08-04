import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AccountCommand } from "../target/types/account_command";
import { assert } from "chai";
import { execSync } from "child_process";

describe("Test CLI account commands", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();

  anchor.setProvider(provider);

  const program = anchor.workspace.AccountCommand as Program<AccountCommand>;

  it("Can fetch and deserialize account using the account command", async () => {
    const myAccount = anchor.web3.Keypair.generate();

    const balance = -2.5;
    const amount = 108;
    const memo = "account test";
    const values = [1, 2, 3, 1000];

    await program.methods
      .initialize(
        balance,
        amount,
        memo,
        values.map((x) => new anchor.BN(x))
      )
      .accountsPartial({
        myAccount: myAccount.publicKey,
        user: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([myAccount])
      .rpc();

    let output: any = {};
    for (let tries = 0; tries < 20; tries++) {
      try {
        output = JSON.parse(
          execSync(
            `anchor account account_command.MyAccount ${myAccount.publicKey}`,
            { stdio: "pipe" }
          ).toString()
        );
        break;
      } catch (e) {
        if (!e.stderr.toString().startsWith("Error: AccountNotFound")) {
          throw e;
        }
      }
    }

    assert.strictEqual(
      output.balance,
      balance,
      "Balance deserialized incorrectly"
    );
    assert.strictEqual(
      output.delegate_pubkey,
      provider.wallet.publicKey.toBase58(),
      "delegatePubkey deserialized incorrectly"
    );
    assert.strictEqual(
      output.sub.state.Confirmed.amount,
      amount,
      "Amount deserialized incorrectly"
    );
    assert.strictEqual(
      output.sub.state.Confirmed.memo,
      memo,
      "Memo deserialized incorrectly"
    );
    for (let i = 0; i < values.length; i++) {
      assert.equal(
        output.sub.values[i],
        values[i],
        "Values deserialized incorrectly"
      );
    }
  });
});
