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

  test("bytes arg named data without raw uses Borsh length-prefixed encoding", () => {
    const idl: Idl = {
      address: "Test111111111111111111111111111111111111111",
      metadata: {
        name: "test",
        version: "0.0.0",
        spec: "0.1.0",
      },
      instructions: [
        {
          name: "upload",
          discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
          accounts: [],
          args: [{ name: "data", type: "bytes" }],
        },
      ],
    };

    const coder = new BorshCoder(idl);
    const payload = Buffer.from([10, 20, 30]);
    const encoded = coder.instruction.encode("upload", { data: payload });
    const afterDisc = encoded.subarray(8);
    assert.strictEqual(afterDisc.readUInt32LE(0), 3);
    assert.deepStrictEqual([...afterDisc.subarray(4)], [10, 20, 30]);
  });

  test("raw: true encodes opaque bytes after discriminator (no length prefix)", () => {
    const idl: Idl = {
      address: "Test111111111111111111111111111111111111111",
      metadata: {
        name: "test",
        version: "0.0.0",
        spec: "0.1.0",
      },
      instructions: [
        {
          name: "upload",
          discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
          accounts: [],
          args: [{ name: "data", type: "bytes" }],
          raw: true,
        },
      ],
    };

    const coder = new BorshCoder(idl);
    const payload = Buffer.from([10, 20, 30]);
    const encoded = coder.instruction.encode("upload", { data: payload });
    assert.deepStrictEqual([...encoded.subarray(8)], [10, 20, 30]);
  });
});
