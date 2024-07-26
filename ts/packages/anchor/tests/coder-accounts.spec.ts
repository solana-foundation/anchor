import * as assert from "assert";
import { BorshCoder, Idl } from "../src";

describe("coder.accounts", () => {
  test("Can encode and decode user-defined accounts, including those with consecutive capital letters", () => {
    const idl: Idl = {
      address: "Test111111111111111111111111111111111111111",
      metadata: {
        name: "basic_0",
        version: "0.0.0",
        spec: "0.1.0",
      },
      instructions: [
        {
          name: "initialize",
          discriminator: [],
          accounts: [],
          args: [],
        },
      ],
      accounts: [
        {
          name: "MemberDAO",
          discriminator: [0, 1, 2, 3, 4, 5, 6, 7],
        },
      ],
      types: [
        {
          name: "MemberDAO",
          type: {
            kind: "struct",
            fields: [
              {
                name: "name",
                type: "string",
              },
            ],
          },
        },
      ],
    };
    const coder = new BorshCoder(idl);

    const memberDAO = {
      name: "test",
    };

    coder.accounts.encode("MemberDAO", memberDAO).then((encoded) => {
      assert.deepEqual(coder.accounts.decode("MemberDAO", encoded), memberDAO);
    });
  });
});
