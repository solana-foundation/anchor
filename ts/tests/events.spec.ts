import { PublicKey } from "@solana/web3.js";
import { EventParser } from "../src/program/event";
import { BorshCoder } from "../src";

describe("Events", () => {
  it("Parses multiple instructions", async () => {
    const logs = [
      "Program 11111111111111111111111111111111 invoke [1]",
      "Program 11111111111111111111111111111111 success",
      "Program J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54 invoke [1]",
      "Program J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54 consumed 17867 of 200000 compute units",
      "Program J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54 success",
    ];
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
    };
    const coder = new BorshCoder(idl);
    const programId = PublicKey.default;
    const eventParser = new EventParser(programId, coder);

    eventParser.parseLogs(logs, () => {
      throw new Error("Should never find logs");
    });
  });
});
