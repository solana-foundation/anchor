import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import { TokenExtensions } from "../target/types/token_extensions";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { it } from "node:test";
import { assert } from "chai";

const TOKEN_2022_PROGRAM_ID = new anchor.web3.PublicKey(
  "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
);

export function associatedAddress({
  mint,
  owner,
}: {
  mint: PublicKey;
  owner: PublicKey;
}): PublicKey {
  return PublicKey.findProgramAddressSync(
    [owner.toBuffer(), TOKEN_2022_PROGRAM_ID.toBuffer(), mint.toBuffer()],
    ASSOCIATED_PROGRAM_ID
  )[0];
}

describe("token extensions", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenExtensions as Program<TokenExtensions>;

  const payer = Keypair.generate();

  it("airdrop payer", async () => {
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(payer.publicKey, 10000000000),
      "confirmed"
    );
  });

  let mint = new Keypair();
  let mintTokenAccount = new Keypair();
  let mintImmutableTokenAccount = new Keypair();

  it("Create mint account test passes", async () => {
    const [extraMetasAccount] = PublicKey.findProgramAddressSync(
      [
        anchor.utils.bytes.utf8.encode("extra-account-metas"),
        mint.publicKey.toBuffer(),
      ],
      program.programId
    );
    await program.methods
      .createMintAccount({
        name: "hello",
        symbol: "hi",
        uri: "https://hi.com",
      })
      .accountsStrict({
        payer: payer.publicKey,
        authority: payer.publicKey,
        receiver: payer.publicKey,
        mint: mint.publicKey,
        mintTokenAccount: mintTokenAccount.publicKey,
        mintImmutableTokenAccount: mintImmutableTokenAccount.publicKey,
        extraMetasAccount: extraMetasAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .signers([mint, mintTokenAccount, mintImmutableTokenAccount, payer])
      .rpc();
  });

  it("mint extension constraints test passes", async () => {
    await program.methods
      .checkMintExtensionsConstraints()
      .accountsStrict({
        authority: payer.publicKey,
        mint: mint.publicKey,
      })
      .signers([payer])
      .rpc();
  });

  it("token account extension constraints test", async () => {
    await program.methods
      .checkTokenAccountExtensionsConstraints()
      .accountsStrict({
        authority: payer.publicKey,
        mint: mint.publicKey,
        mintImmutableTokenAccount: mintImmutableTokenAccount.publicKey,
      })
      .signers([payer])
      .rpc();
  });

  it("missing token account extension constraints test", async () => {
    try {
      await program.methods
      .checkMissingTokenAccountExtensionsConstraints()
      .accountsStrict({
        authority: payer.publicKey,
        mint: mint.publicKey,
        mintTokenAccount: mintTokenAccount.publicKey,
      })
      .signers([payer])
      .rpc();
      assert.fail("Transaction should fail");
    } catch (e) {
      // "Error Code: ConstraintTokenAccountImmutableOwnerExtension. Error Number: 2040. Error Message: A immutable owner extension constraint was violated."
      assert.strictEqual(e.error.errorCode.number, 2040);
    }
  });
});
