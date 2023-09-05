import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";

import { IdlBuildFeatures } from "../target/types/idl_build_features";

describe("idl-build features", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace
    .idlBuildFeatures as anchor.Program<IdlBuildFeatures>;

  it("Can use full module path types", async () => {
    const kp = anchor.web3.Keypair.generate();

    const outerMyStructArg = { u8: 1, u16: 2, u32: 3, u64: new anchor.BN(4) };
    const someModuleMyStructArg = { data: 5 };

    await program.methods
      .fullPath(outerMyStructArg, someModuleMyStructArg)
      .accounts({ account: kp.publicKey })
      .preInstructions([
        await program.account.fullPathAccount.createInstruction(kp),
      ])
      .signers([kp])
      .rpc();

    const fullPathAccount = await program.account.fullPathAccount.fetch(
      kp.publicKey
    );
    assert.strictEqual(fullPathAccount.myStruct.u8, outerMyStructArg.u8);
    assert.strictEqual(fullPathAccount.myStruct.u16, outerMyStructArg.u16);
    assert.strictEqual(fullPathAccount.myStruct.u32, outerMyStructArg.u32);
    assert(fullPathAccount.myStruct.u64.eq(outerMyStructArg.u64));
    assert.deepEqual(fullPathAccount.someModuleMyStruct, someModuleMyStructArg);
  });
});
