import * as assert from "assert";
import { BorshCoder } from "../src";
import { DISCRIMINATOR_SIZE } from "../src/coder/borsh/discriminator";
import { sha256 } from "@noble/hashes/sha256";

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
        encoded.subarray(0, DISCRIMINATOR_SIZE),
        Buffer.from(sha256("account:MemberDAO").slice(0, DISCRIMINATOR_SIZE))
      );
      assert.deepEqual(coder.accounts.decode("MemberDAO", encoded), memberDAO);
    });
  });
});
