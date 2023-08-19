import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { assert } from "chai";

import { Idl } from "../target/types/idl";

describe("IDL", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.idl as Program<Idl>;

  it("Should include `FOO_CONST`", () => {
    assert.isDefined(
      program.idl.constants.find(
        (c) =>
          c.name === "FOO_CONST" && c.type === "u128" && c.value === "1000000"
      )
    );
  });

  it("Should include `BAR_CONST`", () => {
    assert.isDefined(
      program.idl.constants.find(
        (c) => c.name === "BAR_CONST" && c.type === "u8" && c.value === "6"
      )
    );
  });

  it("Should not include `NO_IDL` const", () => {
    // @ts-expect-error
    assert.isUndefined(program.idl.constants.find((c) => c.name === "NO_IDL"));
  });
});
