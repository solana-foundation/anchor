import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { TestProgram } from "../target/types/test_program";

describe("test-program", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.testProgram as Program<TestProgram>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
