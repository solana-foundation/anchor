import * as assert from "assert";
import { createNodeArray } from "typescript";
import { BorshCoder } from "../src";
import { sha256 } from "js-sha256";
import { ACCOUNT_DISCRIMINATOR_SIZE } from "../src/coder/borsh/accounts";

describe("coder.accounts", () => {
  test("Can encode and decode user-defined accounts, including those with consecutive capital letters", () => {
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
      accounts: [
        {
          name: "MemberDAO",
          type: {
            kind: "struct" as const,
            fields: [
              {
                name: "name",
                type: "string" as const,
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
      // start of encoded account = account discriminator
      assert.deepEqual(
        encoded.subarray(0, ACCOUNT_DISCRIMINATOR_SIZE),
        Buffer.from(sha256.digest("account:MemberDAO")).subarray(
          0,
          ACCOUNT_DISCRIMINATOR_SIZE
        )
      );
      assert.deepEqual(coder.accounts.decode("MemberDAO", encoded), memberDAO);
    });
  });
});
