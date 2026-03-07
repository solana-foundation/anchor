import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { NoDeclareId } from "../target/types/no_declare_id";
import { assert } from "chai";

describe("no_declare_id", () => {
  anchor.setProvider(anchor.AnchorProvider.local());
  const program = anchor.workspace.NoDeclareId as Program<NoDeclareId>;

  it("initializes successfully without declare_id!", async () => {
    await program.methods.initialize().rpc();
    assert.ok(true);
  });
});
