import * as anchor from "@anchor-lang/core";
import { Program, AnchorError } from "@anchor-lang/core";
import { expect } from "chai";
import { TestInstructionValidation } from "../target/types/test_instruction_validation";

describe("prove nested instruction buffer consumption issue", () => {
  anchor.setProvider(anchor.AnchorProvider.local());
  const program = anchor.workspace
    .TestInstructionValidation as Program<TestInstructionValidation>;
  const provider = anchor.getProvider() as anchor.AnchorProvider;

  it("Should work when only parent has #[instruction]", async () => {
    // This should work because only parent deserializes instruction args
    const someAccount = anchor.web3.Keypair.generate();

    // Initialize the account first
    await program.methods
      .initializeSomeAccount(new anchor.BN(42))
      .accounts({
        someAccount: someAccount.publicKey,
        user: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([someAccount])
      .rpc();

    // This should succeed - only parent has #[instruction]
    const txSig = await program.methods
      .nestedArgsWorks(new anchor.BN(100), new anchor.BN(200))
      .accounts({
        child: {
          someAccount: someAccount.publicKey,
        },
        user: provider.wallet.publicKey,
      })
      .rpc();

    console.log("✅ nested_args_works succeeded:", txSig);
    expect(txSig).to.be.a("string");
  });

  it("Should FAIL at runtime when both parent and child have #[instruction]", async () => {
    // This demonstrates GitHub issue #2942 - buffer consumption problem
    // Parent deserializes (data: u64, value: u32) = 12 bytes
    // Child tries to deserialize (data: u64, value: u32) = 12 bytes
    // But buffer pointer was already advanced by parent, so child reads wrong data
    const someAccount = anchor.web3.Keypair.generate();

    // Initialize the account first
    await program.methods
      .initializeSomeAccount(new anchor.BN(42))
      .accounts({
        someAccount: someAccount.publicKey,
        user: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([someAccount])
      .rpc();

    // This should FAIL at runtime due to buffer consumption
    try {
      const txSig = await program.methods
        .nestedBothInstruction(new anchor.BN(100), new anchor.BN(200))
        .accounts({
          child: {
            someAccount: someAccount.publicKey,
          },
          user: provider.wallet.publicKey,
        })
        .rpc();

      // If we reach here, the test failed (it should have thrown an error)
      expect.fail("Expected runtime error but transaction succeeded");
    } catch (err) {
      // Expected: InstructionDidNotDeserialize error (error code 102)
      console.log("✅ Caught expected runtime error:", err);
      
      if (err instanceof AnchorError) {
        // Error code 102 = InstructionDidNotDeserialize
        expect(err.error.errorCode.code).to.equal("InstructionDidNotDeserialize");
        console.log("✅ Error code matches: InstructionDidNotDeserialize (102)");
      } else {
        // Might be a different error format, but should still be a deserialization error
        const errStr = err.toString();
        expect(
          errStr.includes("InstructionDidNotDeserialize") ||
          errStr.includes("deserialize") ||
          errStr.includes("102")
        ).to.be.true;
        console.log("✅ Error indicates deserialization failure");
      }
    }
  });
});
