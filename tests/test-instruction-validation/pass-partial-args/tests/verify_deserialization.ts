import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { PublicKey } from "@solana/web3.js";
import { expect } from "chai";
import { TestInstructionValidation } from "../target/types/test_instruction_validation";

describe("Partial #[instruction] deserialization", () => {
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);

  const program = anchor.workspace
    .TestInstructionValidation as Program<TestInstructionValidation>;

  const A = new anchor.BN(100);
  const B = 200;
  const C = new anchor.BN(300);
  const D = 50;
  const user = () => provider.wallet.publicKey;

  const confirmOpts: anchor.web3.ConfirmOptions = { commitment: "confirmed" };

  async function fetchLogs(txSig: string): Promise<string[]> {
    const tx = await provider.connection.getTransaction(txSig, {
      commitment: "confirmed",
      maxSupportedTransactionVersion: 0,
    });
    return tx?.meta?.logMessages ?? [];
  }

  it("#[instruction(a, b)]", async () => {
    const txSig = await program.methods
      .prefixAb(A, B, C, D)
      .accounts({ user: user() })
      .rpc(confirmOpts);

    const logs = await fetchLogs(txSig);
    expect(logs.join("\n")).to.include("prefix_ab: a=100, b=200, c=300, d=50");
  });

  it("#[instruction(c, d)]", async () => {
    const txSig = await program.methods
      .suffixCd(A, B, C, D)
      .accounts({ user: user() })
      .rpc(confirmOpts);

    const logs = await fetchLogs(txSig);
    expect(logs.join("\n")).to.include("suffix_cd: a=100, b=200, c=300, d=50");
  });

  it("#[instruction(b)]", async () => {
    const txSig = await program.methods
      .singleB(A, B, C, D)
      .accounts({ user: user() })
      .rpc(confirmOpts);

    const logs = await fetchLogs(txSig);
    expect(logs.join("\n")).to.include("single_b: a=100, b=200, c=300, d=50");
  });

  it("#[instruction(b, d)]", async () => {
    const txSig = await program.methods
      .singleD(A, B, C, D)
      .accounts({ user: user() })
      .rpc(confirmOpts);

    const logs = await fetchLogs(txSig);
    expect(logs.join("\n")).to.include("single_d: a=100, b=200, c=300, d=50");
  });

  it("#[instruction(x, y)]", async () => {
    const txSig = await program.methods
      .hybridPositional(A, B, C, D)
      .accounts({ user: user() })
      .rpc(confirmOpts);

    const logs = await fetchLogs(txSig);
    expect(logs.join("\n")).to.include(
      "hybrid_positional: a=100, b=200, c=300, d=50"
    );
  });

  it("#[instruction(b)] with seeds", async () => {
    const bValue = 200;
    const bBytes = Buffer.alloc(4);
    bBytes.writeUInt32LE(bValue, 0);

    const [pda] = PublicKey.findProgramAddressSync(
      [Buffer.from("seed_b"), bBytes],
      program.programId
    );

    const txSig = await program.methods
      .withSeeds(A, B, C, D)
      .accounts({ user: user(), pda })
      .rpc(confirmOpts);

    const logs = await fetchLogs(txSig);
    expect(logs.join("\n")).to.include("with_seeds: a=100, b=200, c=300, d=50");
  });
});
