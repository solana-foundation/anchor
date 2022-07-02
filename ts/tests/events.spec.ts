import { PublicKey } from "@solana/web3.js";
import { EventParser } from "../src/program/event";
import { BorshCoder } from "../src";

describe("Events", () => {
  it("Parses multiple instructions", () => {
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

    if (Array.from(eventParser.parseLogs(logs)).length > 0) {
      throw new Error("Should never find logs");
    }
  });
  it("Upgrade event check", () => {
    const logs = [
      "Upgraded program J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54",
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

    if (Array.from(eventParser.parseLogs(logs)).length > 0) {
      throw new Error("Should never find logs");
    }
  });
  it("Find event with different start log.", (done) => {
    const logs = [
      "Upgraded program J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54",
      "Program J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54 invoke [1]",
      "Program log: Instruction: BuyNft",
      "Program 11111111111111111111111111111111 invoke [2]",
      "Program log: UhUxVlc2hGeTBjNPCGmmZjvNSuBOYpfpRPJLfJmTLZueJAmbgEtIMGl9lLKKH6YKy1AQd8lrsdJPPc7joZ6kCkEKlNLKhbUv",
      "Program 11111111111111111111111111111111 success",
      "Program 11111111111111111111111111111111 invoke [2]",
      "Program 11111111111111111111111111111111 success",
      "Program 11111111111111111111111111111111 invoke [2]",
      "Program 11111111111111111111111111111111 success",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]",
      "Program log: Instruction: Transfer",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 2549 of 141128 compute units",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]",
      "Program log: Instruction: CloseAccount",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 1745 of 135127 compute units",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success",
      "Program log: UhUxVlc2hGeTBjNPCGmmZjvNSuBOYpfpRPJLfJmTLZueJAmbgEtIMGl9lLKKH6YKy1AQd8lrsdJPPc7joZ6kCkEKlNLKhbUv",
      "Program J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54 consumed 73106 of 200000 compute units",
      "Program J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54 success",
    ];

    const idl = {
      version: "0.0.0",
      name: "basic_1",
      instructions: [
        {
          name: "initialize",
          accounts: [],
          args: [],
        },
      ],
      events: [
        {
          name: "NftSold",
          fields: [
            {
              name: "nftMintAddress",
              type: "publicKey" as "publicKey",
              index: false,
            },
            {
              name: "accountAddress",
              type: "publicKey" as "publicKey",
              index: false,
            },
          ],
        },
      ],
    };

    const coder = new BorshCoder(idl);
    const programId = new PublicKey(
      "J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54"
    );
    const eventParser = new EventParser(programId, coder);

    const gen = eventParser.parseLogs(logs);
    for (const event of gen) {
      expect(event.name).toEqual("NftSold");
      done();
    }
  });
  it("Find event from logs", (done) => {
    const logs = [
      "Program J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54 invoke [1]",
      "Program log: Instruction: CancelListing",
      "Program log: TRANSFERED SOME TOKENS",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]",
      "Program log: Instruction: Transfer",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 2549 of 182795 compute units",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success",
      "Program log: TRANSFERED SOME TOKENS",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]",
      "Program log: Instruction: CloseAccount",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 1745 of 176782 compute units",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success",
      "Program log: Vtv9xLjCsE60Ati9kl3VVU/5y8DMMeC4LaGdMLkX8WU+G59Wsi3wfky8rnO9otGb56CTRerWx3hB5M/SlRYBdht0fi+crAgFYsJcx2CHszpSWRkXNxYQ6DxQ/JqIvKnLC/8Mln7310A=",
      "Program J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54 consumed 31435 of 200000 compute units",
      "Program J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54 success",
    ];

    const idl = {
      version: "0.0.0",
      name: "basic_2",
      instructions: [
        {
          name: "cancelListing",
          accounts: [
            {
              name: "globalState",
              isMut: true,
              isSigner: false,
            },
            {
              name: "nftHolderAccount",
              isMut: true,
              isSigner: false,
            },
            {
              name: "listingAccount",
              isMut: true,
              isSigner: false,
            },
            {
              name: "nftAssociatedAccount",
              isMut: true,
              isSigner: false,
            },
            {
              name: "signer",
              isMut: true,
              isSigner: true,
            },
            {
              name: "tokenProgram",
              isMut: false,
              isSigner: false,
            },
          ],
          args: [],
        },
      ],
      events: [
        {
          name: "ListingClosed",
          fields: [
            {
              name: "initializer",
              type: "publicKey" as "publicKey",
              index: false,
            },
            {
              name: "nftMintAddress",
              type: "publicKey" as "publicKey",
              index: false,
            },
            {
              name: "accountAddress",
              type: "publicKey" as "publicKey",
              index: false,
            },
          ],
        },
      ],
    };

    const coder = new BorshCoder(idl);
    const programId = new PublicKey(
      "J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54"
    );
    const eventParser = new EventParser(programId, coder);

    const gen = eventParser.parseLogs(logs);
    for (const event of gen) {
      expect(event.name).toEqual("ListingClosed");
      done();
    }
  });
  it("Listen to different program and send other program logs with same name", () => {
    const logs = [
      "Program 5VcVB7jEjdWJBkriXxayCrUUkwfhrPK3rXtnkxxUvMFP invoke [1]",
      "Program log: Instruction: CancelListing",
      "Program log: TRANSFERED SOME TOKENS",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]",
      "Program log: Instruction: Transfer",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 2549 of 182795 compute units",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success",
      "Program log: TRANSFERED SOME TOKENS",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]",
      "Program log: Instruction: CloseAccount",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 1745 of 176782 compute units",
      "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success",
      "Program log: Vtv9xLjCsE60Ati9kl3VVU/5y8DMMeC4LaGdMLkX8WU+G59Wsi3wfky8rnO9otGb56CTRerWx3hB5M/SlRYBdht0fi+crAgFYsJcx2CHszpSWRkXNxYQ6DxQ/JqIvKnLC/8Mln7310A=",
      "Program 5VcVB7jEjdWJBkriXxayCrUUkwfhrPK3rXtnkxxUvMFP consumed 31435 of 200000 compute units",
      "Program 5VcVB7jEjdWJBkriXxayCrUUkwfhrPK3rXtnkxxUvMFP success",
    ];

    const idl = {
      version: "0.0.0",
      name: "basic_2",
      instructions: [],
      events: [
        {
          name: "ListingClosed",
          fields: [
            {
              name: "initializer",
              type: "publicKey" as "publicKey",
              index: false,
            },
            {
              name: "nftMintAddress",
              type: "publicKey" as "publicKey",
              index: false,
            },
            {
              name: "accountAddress",
              type: "publicKey" as "publicKey",
              index: false,
            },
          ],
        },
      ],
    };

    const coder = new BorshCoder(idl);
    const programId = new PublicKey(
      "J2XMGdW2qQLx7rAdwWtSZpTXDgAQ988BLP9QTgUZvm54"
    );
    const eventParser = new EventParser(programId, coder);

    if (Array.from(eventParser.parseLogs(logs)).length > 0) {
      throw new Error("Should never find logs");
    }
  });
});
