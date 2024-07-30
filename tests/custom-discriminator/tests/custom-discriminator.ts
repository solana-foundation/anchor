import * as anchor from "@coral-xyz/anchor";
import assert from "assert";

import type { CustomDiscriminator } from "../target/types/custom_discriminator";

describe("custom-discriminator", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program: anchor.Program<CustomDiscriminator> =
    anchor.workspace.customDiscriminator;

  describe("Can use custom instruction discriminators", () => {
    const testCommon = async (ixName: keyof typeof program["methods"]) => {
      const tx = await program.methods[ixName]().transaction();

      // Verify discriminator
      const ix = program.idl.instructions.find((ix) => ix.name === ixName)!;
      assert(ix.discriminator.length < 8);
      const data = tx.instructions[0].data;
      assert(data.equals(Buffer.from(ix.discriminator)));

      // Verify tx runs
      await program.provider.sendAndConfirm!(tx);
    };

    it("Integer", () => testCommon("int"));
    it("Array", () => testCommon("array"));
    it("Byte string", () => testCommon("byteStr"));
    it("Constant", () => testCommon("constant"));
    it("Const Fn", () => testCommon("constFn"));
  });
});
