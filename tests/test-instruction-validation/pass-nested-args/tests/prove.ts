import * as anchor from "@anchor-lang/core";
import { Program, AnchorError } from "@anchor-lang/core";
import { expect } from "chai";
import { TestInstructionValidation } from "../target/types/test_instruction_validation";

describe("nested instruction buffer consumption fix", () => {
  anchor.setProvider(anchor.AnchorProvider.local());
  const program = anchor.workspace
    .TestInstructionValidation as Program<TestInstructionValidation>;
  const provider = anchor.getProvider() as anchor.AnchorProvider;

  it("Should succeed when both parent and child have #[instruction]", async () => {
    const someAccount = anchor.web3.Keypair.generate();

    await program.methods
      .initializeSomeAccount(new anchor.BN(42))
      .accounts({
        someAccount: someAccount.publicKey,
        user: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([someAccount])
      .rpc();

    const txSig = await program.methods
      .nestedBothInstruction(new anchor.BN(100), new anchor.BN(200))
      .accounts({
        child: {
          someAccount: someAccount.publicKey,
        },
        user: provider.wallet.publicKey,
      })
      .rpc();

    expect(txSig).to.be.a("string");
  });
});
