import * as anchor from "@coral-xyz/anchor";
import assert from "assert";

import type { DeclareProgram } from "../target/types/declare_program";
import type { External } from "../target/types/external";

describe("declare-program", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program: anchor.Program<DeclareProgram> =
    anchor.workspace.declareProgram;
  const externalProgram: anchor.Program<External> = anchor.workspace.external;

  it("Can CPI", async () => {
    const { pubkeys } = await externalProgram.methods.init().rpcAndKeys();

    const value = 5;
    await program.methods
      .cpi(value)
      .accounts({ cpiMyAccount: pubkeys.myAccount })
      .rpc();

    const myAccount = await externalProgram.account.myAccount.fetch(
      pubkeys.myAccount
    );
    assert.strictEqual(myAccount.field, value);
  });
});
