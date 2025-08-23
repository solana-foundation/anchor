import * as anchor from "@coral-xyz/anchor";
import { AnchorError, Program } from "@coral-xyz/anchor";
import { CustomProgram } from "../target/types/custom_program";
import { assert } from "chai";

const CUSTOM_PROGRAM_ID = "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY";

// This was an Executable Data account for our custom program which is not executable
const NON_EXECUTABLE_ACCOUNT_ID =
  "9cxLzxjrTeodcbaEU3KCNGE1a4yFZEcdJ7uEXN378S4U";

describe("custom_program", () => {
  anchor.setProvider(anchor.AnchorProvider.local());
  const program = anchor.workspace.CustomProgram as Program<CustomProgram>;

  it("Should pass test program validation", async () => {
    try {
      await program.methods
        .testProgramValidation()
        .accounts({
          genericProgram: new anchor.web3.PublicKey(CUSTOM_PROGRAM_ID),
          systemProgram: anchor.web3.SystemProgram.programId,
          customProgramInput: program.programId,
        })
        .rpc();
      assert.ok(true);
    } catch (_err) {
      console.log(_err);
      assert(false);
    }
  });

  it("Should fail test program validation", async () => {
    try {
      await program.methods
        .testProgramValidation()
        .accounts({
          genericProgram: new anchor.web3.PublicKey(NON_EXECUTABLE_ACCOUNT_ID),
          systemProgram: anchor.web3.SystemProgram.programId,
          customProgramInput: program.programId,
        })
        .rpc();
      assert.ok(false);
    }
    catch (_err) {
      assert.ok(true);
      assert.isTrue(_err instanceof AnchorError);
      const err: AnchorError = _err;
      assert.strictEqual(err.error.errorCode.number, 3009);
      assert.strictEqual(
        err.error.errorMessage,
        "Program account is not executable"
      );
    }
  });
});
