import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { assert } from "chai";

import { Idl } from "../target/types/idl";

describe("IDL", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.idl as Program<Idl>;

  it("Includes constants that use `#[constant]` macro", () => {
    const checkDefined = (
      cb: (constant: typeof program["idl"]["constants"][number]) => boolean
    ) => {
      program.idl.constants.find((c) => cb(c));
    };

    checkDefined((c) => c.name === "U8" && c.type === "u8" && c.value === "6");
    checkDefined(
      (c) => c.name === "I128" && c.type === "i128" && c.value === "1000000"
    );
    checkDefined(
      (c) => c.name === "BYTE_STR" && c.type === "u8" && c.value === "116"
    );
    checkDefined(
      (c) =>
        c.name === "BYTES_STR" &&
        c.type === "bytes" &&
        c.value === "[116, 101, 115, 116]"
    );
  });

  it("Does not include constants that does not use `#[constant]` macro ", () => {
    // @ts-expect-error
    assert.isUndefined(program.idl.constants.find((c) => c.name === "NO_IDL"));
  });
});
