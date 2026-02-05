import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { assert } from "chai";
import { LogComputeUnits } from "../target/types/log_compute_units";

// Helper to fetch transaction with retries
async function getTransactionWithRetry(
  connection: anchor.web3.Connection,
  signature: string,
  maxRetries = 10
) {
  for (let i = 0; i < maxRetries; i++) {
    const txDetails = await connection.getTransaction(signature, {
      commitment: "confirmed",
      maxSupportedTransactionVersion: 0,
    });
    if (txDetails) {
      return txDetails;
    }
    // Wait before retry
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(
    `Transaction ${signature} not found after ${maxRetries} retries`
  );
}

describe("log-compute-units", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);
  const program = anchor.workspace.LogComputeUnits as Program<LogComputeUnits>;

  it("Logs compute units during initialize", async () => {
    const data = anchor.web3.Keypair.generate();

    const tx = await program.methods
      .initialize()
      .accounts({
        data: data.publicKey,
        payer: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([data])
      .rpc({ commitment: "confirmed" });

    console.log("Initialize transaction signature:", tx);

    // Fetch the transaction to check logs (with retry)
    const txDetails = await getTransactionWithRetry(provider.connection, tx);

    console.log("\n--- Transaction Logs ---");
    txDetails.meta?.logMessages?.forEach((log) => console.log(log));
    console.log("------------------------\n");

    // Verify compute unit logs are present
    const logs = txDetails.meta?.logMessages || [];
    const anchorLogs = logs.filter(
      (log) => log.includes("anchor-compute:") && log.includes("units")
    );
    const customLogs = logs.filter(
      (log) => log.includes("custom-compute:") && log.includes("units")
    );

    // Should have 6 anchor compute logs + 1 custom log from user handler
    assert.ok(
      anchorLogs.length >= 6,
      `Expected at least 6 anchor compute logs, got ${anchorLogs.length}`
    );
    assert.ok(
      customLogs.length >= 1,
      `Expected at least 1 custom compute log, got ${customLogs.length}`
    );
  });

  it("Logs compute units during update", async () => {
    const data = anchor.web3.Keypair.generate();

    // First initialize
    await program.methods
      .initialize()
      .accounts({
        data: data.publicKey,
        payer: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([data])
      .rpc({ commitment: "confirmed" });

    // Then update
    const tx = await program.methods
      .update(new anchor.BN(100))
      .accounts({
        data: data.publicKey,
      })
      .rpc({ commitment: "confirmed" });

    console.log("Update transaction signature:", tx);

    // Fetch the transaction to check logs (with retry)
    const txDetails = await getTransactionWithRetry(provider.connection, tx);

    console.log("\n--- Transaction Logs ---");
    txDetails.meta?.logMessages?.forEach((log) => console.log(log));
    console.log("------------------------\n");

    // Verify compute unit logs are present
    const logs = txDetails.meta?.logMessages || [];
    const anchorLogs = logs.filter(
      (log) => log.includes("anchor-compute:") && log.includes("units")
    );
    const customLogs = logs.filter(
      (log) => log.includes("custom-compute:") && log.includes("units")
    );

    // Should have 6 anchor compute logs + 1 custom log from user handler
    assert.ok(
      anchorLogs.length >= 6,
      `Expected at least 6 anchor compute logs, got ${anchorLogs.length}`
    );
    assert.ok(
      customLogs.length >= 1,
      `Expected at least 1 custom compute log, got ${customLogs.length}`
    );

    // Verify the data was actually updated
    const dataAccount = await program.account.data.fetch(data.publicKey);
    assert.equal(dataAccount.value.toNumber(), 100);
  });
});
