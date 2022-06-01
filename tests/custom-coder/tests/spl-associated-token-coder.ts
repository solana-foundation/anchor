import * as anchor from "@project-serum/anchor";
import { Native, Spl } from "@project-serum/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import * as assert from "assert";
import BN from "bn.js";

describe("spl-associated-token-coder", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Client.
  const program = Spl.associatedToken();
  const systemProgram = Native.system();
  const tokenProgram = Spl.token();

  it("Creates an account", async () => {
    // arrange
    const mintKeypair = Keypair.generate();
    const mintDecimals = 6;
    const mintSize = tokenProgram.coder.accounts.size(
      tokenProgram.idl.accounts[0]
    );
    const mintRentExemption =
      await provider.connection.getMinimumBalanceForRentExemption(mintSize);
    const [associatedToken] = await PublicKey.findProgramAddress(
      [
        provider.publicKey.toBuffer(),
        tokenProgram.programId.toBuffer(),
        mintKeypair.publicKey.toBuffer(),
      ],
      program.programId
    );

    // act
    await program.methods
      .create()
      .accounts({
        authority: provider.wallet.publicKey,
        mint: mintKeypair.publicKey,
        owner: provider.wallet.publicKey,
        associatedAccount: associatedToken,
      })
      .preInstructions(
        await Promise.all([
          systemProgram.methods
            .createAccount(
              new BN(mintRentExemption),
              new BN(mintSize),
              tokenProgram.programId
            )
            .accounts({
              from: provider.wallet.publicKey,
              to: mintKeypair.publicKey,
            })
            .instruction(),
          tokenProgram.methods
            .initializeMint(mintDecimals, provider.wallet.publicKey, null)
            .accounts({
              mint: mintKeypair.publicKey,
            })
            .instruction(),
        ])
      )
      .signers([mintKeypair])
      .rpc();
    // assert
    const tokenAccount = await tokenProgram.account.token.fetch(
      associatedToken
    );
    assert.ok(tokenAccount.mint.equals(mintKeypair.publicKey));
  });
});
