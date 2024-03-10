import * as assert from "assert";
import { BorshCoder } from "../src";
import { Idl, IdlType } from "../src/idl";
import { toInstruction } from "../src/program/common";

describe("coder.instructions", () => {
  test("Can encode and decode type aliased instruction arguments (byte array)", () => {
    const idl: Idl = {
      address: "Test111111111111111111111111111111111111111",
      metadata: {
        name: "test",
        version: "0.0.0",
        spec: "0.1.0",
      },
      instructions: [
        {
          name: "initialize",
          discriminator: [0, 1, 2, 3, 4, 5, 6, 7],
          accounts: [],
          args: [
            {
              name: "arg",
              type: {
                defined: {
                  name: "AliasTest",
                },
              },
            },
          ],
        },
      ],
      types: [
        {
          name: "AliasTest",
          type: {
            kind: "type",
            alias: {
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
    const decoded = coder.instruction.decode(encoded);

    assert.deepStrictEqual(decoded?.data[idlIx.args[0].name], expected);
  });
});
