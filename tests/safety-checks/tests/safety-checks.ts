import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { SafetyChecks } from "../target/types/safety_checks";

describe("safety-checks", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.SafetyChecks as Program<SafetyChecks>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.rpc.initialize({});
    console.log("Your transaction signature", tx);
  });
});
