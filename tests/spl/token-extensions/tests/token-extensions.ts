import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { TokenExtensions } from "../target/types/token_extensions";
import { ASSOCIATED_PROGRAM_ID } from "@anchor-lang/core/dist/cjs/utils/token";
import {
  createInitializeAccountInstruction,
  createMint,
  ExtensionType,
  getAccountLen,
} from "@solana/spl-token";
import { it } from "node:test";

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
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
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

  describe("group_pointer_update", () => {
    let groupPointerMint = new Keypair();

    it("Create mint with group pointer extension", async () => {
      await program.methods
        .createGroupPointerMint()
        .accountsStrict({
          payer: payer.publicKey,
          authority: payer.publicKey,
          mint: groupPointerMint.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .signers([payer, groupPointerMint])
        .rpc();
    });

    it("Update group pointer via CPI succeeds", async () => {
      const newGroupAddress = Keypair.generate().publicKey;
      await program.methods
        .updateGroupPointer(newGroupAddress)
        .accountsStrict({
          authority: payer.publicKey,
          mint: groupPointerMint.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .signers([payer])
        .rpc();
    });

    it("Update group pointer to None via CPI succeeds", async () => {
      await program.methods
        .updateGroupPointer(null)
        .accountsStrict({
          authority: payer.publicKey,
          mint: groupPointerMint.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .signers([payer])
        .rpc();
    });
  });

  describe("cpi_guard", () => {
    let cpiGuardMint: PublicKey;
    let enableAccount = Keypair.generate();
    let disableAccount = Keypair.generate();

    async function createCpiGuardTokenAccount(
      tokenAccountKeypair: Keypair
    ): Promise<void> {
      const accountLen = getAccountLen([ExtensionType.CpiGuard]);
      const lamports =
        await provider.connection.getMinimumBalanceForRentExemption(accountLen);

      const tx = new Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: payer.publicKey,
          newAccountPubkey: tokenAccountKeypair.publicKey,
          space: accountLen,
          lamports,
          programId: TOKEN_2022_PROGRAM_ID,
        }),
        createInitializeAccountInstruction(
          tokenAccountKeypair.publicKey,
          cpiGuardMint,
          payer.publicKey,
          TOKEN_2022_PROGRAM_ID
        )
      );

      await sendAndConfirmTransaction(
        provider.connection,
        tx,
        [payer, tokenAccountKeypair],
        { commitment: "confirmed" }
      );
    }

    it("Create mint and token accounts with CPI Guard extension", async () => {
      cpiGuardMint = await createMint(
        provider.connection,
        payer,
        payer.publicKey,
        null,
        9,
        Keypair.generate(),
        { commitment: "confirmed" },
        TOKEN_2022_PROGRAM_ID
      );

      await createCpiGuardTokenAccount(enableAccount);
      await createCpiGuardTokenAccount(disableAccount);
    });

    it("Enable CPI Guard via CPI succeeds", async () => {
      await program.methods
        .enableCpiGuard()
        .accountsStrict({
          authority: payer.publicKey,
          tokenAccount: enableAccount.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .signers([payer])
        .rpc();
    });

    it("Disable CPI Guard via CPI succeeds", async () => {
      // Uses a separate account where guard is not active,
      // since an active CPI Guard blocks disable via CPI
      await program.methods
        .disableCpiGuard()
        .accountsStrict({
          authority: payer.publicKey,
          tokenAccount: disableAccount.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .signers([payer])
        .rpc();
    });
  });
});
