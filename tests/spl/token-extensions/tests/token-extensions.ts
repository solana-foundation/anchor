import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { TokenExtensions } from "../target/types/token_extensions";
import { it } from "node:test";
import {
  TOKEN_2022_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

export function associatedAddress({
  mint,
  owner,
}: {
  mint: PublicKey;
  owner: PublicKey;
}): PublicKey {
  return PublicKey.findProgramAddressSync(
    [owner.toBuffer(), TOKEN_2022_PROGRAM_ID.toBuffer(), mint.toBuffer()],
    ASSOCIATED_TOKEN_PROGRAM_ID
  )[0];
}

describe("token extensions", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenExtensions as Program<TokenExtensions>;

  const payer = Keypair.generate();

  it("airdrop payer", async () => {
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        payer.publicKey,
        10 * LAMPORTS_PER_SOL
      ),
      "confirmed"
    );
  });

  let mint = new Keypair();

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
        mintTokenAccount: associatedAddress({
          mint: mint.publicKey,
          owner: payer.publicKey,
        }),
        extraMetasAccount: extraMetasAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .signers([mint, payer])
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
});
