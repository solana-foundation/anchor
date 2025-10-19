import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Example } from "../target/types/example";
import { assert } from "chai";

describe("multiple-scripts", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Example as Program<Example>;

  it("Initializes successfully", async () => {
    // Call the initialize instruction
    const tx = await program.methods
      .initialize()
      .rpc();

    console.log("Transaction signature:", tx);
    assert.ok(tx, "Transaction should have a signature");
  });

  it("Can be called multiple times", async () => {
    // Call initialize again to demonstrate it can be called multiple times
    const tx = await program.methods
      .initialize()
      .rpc();

    console.log("Second transaction signature:", tx);
    assert.ok(tx, "Second transaction should also succeed");
  });
});

