import * as assert from "assert";
import { BorshCoder } from "../src";

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
});
