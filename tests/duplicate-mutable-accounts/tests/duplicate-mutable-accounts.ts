import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorError } from "@coral-xyz/anchor";
import { DuplicateMutableAccounts } from "../target/types/duplicate_mutable_accounts";
import { assert } from "chai";

describe("duplicate-mutable-accounts", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const program = anchor.workspace
    .DuplicateMutableAccounts as Program<DuplicateMutableAccounts>;

  // Payer used by #[account(init, payer = user, ...)]
  const user_wallet = anchor.web3.Keypair.generate();

  // Two regular system accounts to hold Counter state (must sign on init)
  const dataAccount1 = anchor.web3.Keypair.generate();
  const dataAccount2 = anchor.web3.Keypair.generate();

  it("Initialize accounts", async () => {
    // 1) Fund user_wallet so it can pay rent
    const airdropSig = await provider.connection.requestAirdrop(
      user_wallet.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(airdropSig);

    // 2) Create & init dataAccount1 (must sign with dataAccount1)
    await program.methods
      .initialize(new anchor.BN(100))
      .accounts({
        dataAccount: dataAccount1.publicKey,
        user: user_wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user_wallet, dataAccount1]) // <- include the new account keypair
      .rpc();

    // 3) Create & init dataAccount2
    await program.methods
      .initialize(new anchor.BN(300))
      .accounts({
        dataAccount: dataAccount2.publicKey,
        user: user_wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user_wallet, dataAccount2]) // <- include the new account keypair
      .rpc();
  });

  it("Should fail with duplicate mutable accounts", async () => {
    // Ensure the accounts are initialized
    const account1 = await program.account.counter.fetch(
      dataAccount1.publicKey
    );
    const account2 = await program.account.counter.fetch(
      dataAccount2.publicKey
    );
    assert.strictEqual(account1.count.toNumber(), 100);
    assert.strictEqual(account2.count.toNumber(), 300);

    try {
      await program.methods
        .failsDuplicateMutable()
        .accounts({
          account1: dataAccount1.publicKey,
          account2: dataAccount1.publicKey, // <- SAME account to trigger the check
        })
        .rpc();
      assert.fail("Expected duplicate mutable violation");
    } catch (e) {
      assert.instanceOf(e, AnchorError);
      const err = e as AnchorError;
      assert.strictEqual(
        err.error.errorCode.code,
        "ConstraintDuplicateMutableAccount"
      );
      assert.strictEqual(err.error.errorCode.number, 2040);
    }
  });

  it("Should succeed with duplicate mutable accounts", async () => {
    // This instruction MUST have `#[account(mut, dup)]` on at least one account
    await program.methods
      .allowsDuplicateMutable()
      .accounts({
        account1: dataAccount1.publicKey,
        account2: dataAccount1.publicKey, // same account allowed via `dup`
      })
      .rpc();
    assert.ok(true);
  });
});
