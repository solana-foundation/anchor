import assert from "assert";
import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { Callee } from "../target/types/callee";
import { Caller } from "../target/types/caller";

const { SystemProgram } = anchor.web3;

describe("realloc-cpi-bug", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const calleeProgram = anchor.workspace.Callee as Program<Callee>;
  const callerProgram = anchor.workspace.Caller as Program<Caller>;

  let dataAccount: anchor.web3.PublicKey;

  before(async () => {
    [dataAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("data")],
      calleeProgram.programId
    );
  });

  it("initializes the data account", async () => {
    await calleeProgram.methods
      .initialize()
      .accounts({
        authority: provider.wallet.publicKey,
        dataAccount,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const account = await calleeProgram.account.dataAccount.fetch(dataAccount);
    assert.strictEqual(account.data.length, 1);
  });

  it("can realloc via direct call", async () => {
    await calleeProgram.methods
      .realloc(100)
      .accounts({
        authority: provider.wallet.publicKey,
        dataAccount,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const account = await calleeProgram.account.dataAccount.fetch(dataAccount);
    assert.strictEqual(account.data.length, 100);
  });

  it("can realloc via CPI (caller -> callee)", async () => {
    // This is the key test case: CPI depth 2 realloc.
    // Before the fix, this would fail with:
    // "sum of account balances before and after instruction do not match"
    await callerProgram.methods
      .callRealloc(200)
      .accounts({
        authority: provider.wallet.publicKey,
        dataAccount,
        calleeProgram: calleeProgram.programId,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const account = await calleeProgram.account.dataAccount.fetch(dataAccount);
    assert.strictEqual(account.data.length, 200);
  });

  it("can realloc to smaller size via CPI", async () => {
    // Test shrinking also works via CPI (this doesn't use system_program::transfer,
    // but verifies the realloc path doesn't break).
    await callerProgram.methods
      .callRealloc(50)
      .accounts({
        authority: provider.wallet.publicKey,
        dataAccount,
        calleeProgram: calleeProgram.programId,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const account = await calleeProgram.account.dataAccount.fetch(dataAccount);
    assert.strictEqual(account.data.length, 50);
  });

  it("can realloc back to larger size via CPI", async () => {
    // Grow again after shrinking, to verify the transfer path works at CPI depth 2.
    await callerProgram.methods
      .callRealloc(500)
      .accounts({
        authority: provider.wallet.publicKey,
        dataAccount,
        calleeProgram: calleeProgram.programId,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const account = await calleeProgram.account.dataAccount.fetch(dataAccount);
    assert.strictEqual(account.data.length, 500);
  });
});
