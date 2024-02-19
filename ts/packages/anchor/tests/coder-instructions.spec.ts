import * as assert from "assert";
import { BorshCoder } from "../src";
import { IdlType } from "../src/idl";
import { toInstruction } from "../src/program/common";

describe("coder.instructions", () => {
  test("Can encode and decode type aliased instruction arguments (byte array)", () => {
    const idl = {
      version: "0.1.0",
      name: "test",
      instructions: [
        {
          name: "initialize",
          accounts: [],
          args: [
            {
              name: "arg",
              type: {
                defined: "AliasTest",
              },
            },
          ],
        },
      ],
      types: [
        {
          name: "AliasTest",
          type: {
            kind: "alias" as const,
            value: {
              array: ["u8", 3] as [IdlType, number],
            },
          },
        },
      ],
    };

    const idlIx = idl.instructions[0];
    const expected = [1, 2, 3];

    const coder = new BorshCoder(idl);
    const ix = toInstruction(idlIx, expected);

    const encoded = coder.instruction.encode(idlIx.name, ix);
    const decoded = coder.instruction.decode(encoded, "hex", idlIx.name);

    assert.deepStrictEqual(decoded?.data[idlIx.args[0].name], expected);
  });
});
