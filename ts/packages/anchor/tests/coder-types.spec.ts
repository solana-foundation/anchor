import * as assert from "assert";
import { BorshCoder } from "../src";
import BN from "bn.js";

describe("coder.types", () => {
  test("Can encode and decode user-defined types", () => {
    const idl = {
      version: "0.0.0",
      name: "basic_0",
      instructions: [
        {
          name: "initialize",
          accounts: [],
          args: [],
        },
      ],
      types: [
        {
          name: "MintInfo",
          type: {
            kind: "struct" as const,
            fields: [
              {
                name: "minted",
                type: "bool" as const,
              },
              {
                name: "metadataUrl",
                type: "string" as const,
              },
            ],
          },
        },
      ],
    };
    const coder = new BorshCoder(idl);

    const mintInfo = {
      minted: true,
      metadataUrl: "hello",
    };
    const encoded = coder.types.encode("MintInfo", mintInfo);

    assert.deepEqual(coder.types.decode("MintInfo", encoded), mintInfo);
  });

  test("Can encode and decode 256-bit integers", () => {
    const idl = {
      version: "0.0.0",
      name: "basic_0",
      instructions: [
        {
          name: "initialize",
          accounts: [],
          args: [],
        },
      ],
      types: [
        {
          name: "IntegerTest",
          type: {
            kind: "struct" as const,
            fields: [
              {
                name: "unsigned",
                type: "u256" as const,
              },
              {
                name: "signed",
                type: "i256" as const,
              },
            ],
          },
        },
      ],
    };

    const testing = {
      unsigned: new BN(2588012355),
      signed: new BN(-93842345),
    };

    const coder = new BorshCoder(idl);
    const encoded = coder.types.encode("IntegerTest", testing);
    assert.strictEqual(
      coder.types.decode("IntegerTest", encoded).toString(),
      testing.toString()
    );
  });
});
