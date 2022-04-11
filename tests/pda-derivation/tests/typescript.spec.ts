import * as anchor from "@project-serum/anchor";
import BN from "bn.js";
import { Keypair } from "@solana/web3.js";
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import { Program } from "@project-serum/anchor";
import { PdaDerivation } from "../target/types/pda_derivation";
import { expect } from "chai";
const encode = anchor.utils.bytes.utf8.encode;

describe("typescript", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.PdaDerivation as Program<PdaDerivation>;
  const base = Keypair.generate();
  const dataKey = Keypair.generate();
  const data = new BN(1);
  const seedA = 4;

  it("Inits the base account", async () => {
    await program.methods
      .initBase(data, dataKey.publicKey)
      .accounts({
        base: base.publicKey,
      })
      .signers([base])
      .rpc();
  });

  it("Inits the derived accounts", async () => {
    const MY_SEED = "hi";
    const MY_SEED_STR = "hi";
    const MY_SEED_U8 = 1;
    const MY_SEED_U32 = 2;
    const MY_SEED_U64 = 3;
    const expectedPDAKey = findProgramAddressSync(
      [
        Buffer.from([seedA]),
        encode("another-seed"),
        encode("test"),
        base.publicKey.toBuffer(),
        base.publicKey.toBuffer(),
        encode(MY_SEED),
        encode(MY_SEED_STR),
        Buffer.from([MY_SEED_U8]),
        new anchor.BN(MY_SEED_U32).toArrayLike(Buffer, "le", 4),
        new anchor.BN(MY_SEED_U64).toArrayLike(Buffer, "le", 8),
        new anchor.BN(data).toArrayLike(Buffer, "le", 8),
        dataKey.publicKey.toBuffer(),
      ],
      program.programId
    )[0];

    const tx = program.methods.initMyAccount(seedA).accounts({
      base: base.publicKey,
      base2: base.publicKey,
    });

    const keys = await tx.pubkeys();
    expect(keys.account.equals(expectedPDAKey)).is.true;

    await tx.rpc();

    const actualData = (await program.account.myAccount.fetch(expectedPDAKey))
      .data;
    expect(actualData.toNumber()).is.equal(1337);
  });
});
