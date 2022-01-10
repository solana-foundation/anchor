import * as anchor from "@project-serum/anchor";
import { Spl } from "@project-serum/anchor";
import * as assert from "assert";
import BN from "bn.js";
import { Keypair, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";

describe("custom-coder", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  // Client.
  const program = Spl.token();

  // Constants.
  const mintKeypair = Keypair.generate();
  const aliceTokenKeypair = Keypair.generate();
  const bobTokenKeypair = Keypair.generate();
  const rent = SYSVAR_RENT_PUBKEY;

  it("Creates a mint", async () => {
    await program.rpc.initializeMint(
      6,
      program.provider.wallet.publicKey,
      null,
      {
        accounts: {
          mint: mintKeypair.publicKey,
          rent,
        },
        signers: [mintKeypair],
        preInstructions: [
          await program.account.mint.createInstruction(mintKeypair),
        ],
      }
    );
    const mintAccount = await program.account.mint.fetch(mintKeypair.publicKey);
    assert.ok(
      mintAccount.mintAuthority.equals(program.provider.wallet.publicKey)
    );
    assert.ok(mintAccount.freezeAuthority === null);
    assert.ok(mintAccount.decimals === 6);
    assert.ok(mintAccount.isInitialized);
    assert.ok(mintAccount.supply.toNumber() === 0);
  });

  it("Creates a token account for alice", async () => {
    await program.rpc.initializeAccount({
      accounts: {
        account: aliceTokenKeypair.publicKey,
        mint: mintKeypair.publicKey,
        authority: program.provider.wallet.publicKey,
        rent,
      },
      signers: [aliceTokenKeypair],
      preInstructions: [
        await program.account.token.createInstruction(aliceTokenKeypair),
      ],
    });
    const token = await program.account.token.fetch(
      aliceTokenKeypair.publicKey
    );
    assert.ok(token.authority.equals(program.provider.wallet.publicKey));
    assert.ok(token.mint.equals(mintKeypair.publicKey));
    assert.ok(token.amount.toNumber() === 0);
    assert.ok(token.delegate === null);
    assert.ok(token.state === 0);
    assert.ok(token.isNative === null);
    assert.ok(token.delegatedAmount.toNumber() === 0);
    assert.ok(token.closeAuthority === null);
  });

  it("Mints a token to alice", async () => {
    await program.rpc.mintTo(new BN(2), {
      accounts: {
        mint: mintKeypair.publicKey,
        to: aliceTokenKeypair.publicKey,
        authority: program.provider.wallet.publicKey,
      },
    });

    const token = await program.account.token.fetch(
      aliceTokenKeypair.publicKey
    );
    const mint = await program.account.mint.fetch(mintKeypair.publicKey);
    assert.ok(token.amount.toNumber() === 2);
    assert.ok(mint.supply.toNumber() === 2);
  });

  it("Creates a token for bob", async () => {
    await program.rpc.initializeAccount({
      accounts: {
        account: bobTokenKeypair.publicKey,
        mint: mintKeypair.publicKey,
        authority: program.provider.wallet.publicKey,
        rent,
      },
      signers: [bobTokenKeypair],
      preInstructions: [
        await program.account.token.createInstruction(bobTokenKeypair),
      ],
    });
  });

  it("Transfer a token from alice to bob", async () => {
    await program.rpc.transfer(new BN(1), {
      accounts: {
        source: aliceTokenKeypair.publicKey,
        destination: bobTokenKeypair.publicKey,
        authority: program.provider.wallet.publicKey,
      },
    });
    const aliceToken = await program.account.token.fetch(
      aliceTokenKeypair.publicKey
    );
    const bobToken = await program.account.token.fetch(
      bobTokenKeypair.publicKey
    );
    assert.ok(aliceToken.amount.toNumber() === 1);
    assert.ok(bobToken.amount.toNumber() === 1);
  });

  it("Alice burns a token", async () => {
    await program.rpc.burn(new BN(1), {
      accounts: {
        source: aliceTokenKeypair.publicKey,
        mint: mintKeypair.publicKey,
        authority: program.provider.wallet.publicKey,
      },
    });
    const aliceToken = await program.account.token.fetch(
      aliceTokenKeypair.publicKey
    );
    const mint = await program.account.mint.fetch(mintKeypair.publicKey);
    assert.ok(aliceToken.amount.toNumber() === 0);
    assert.ok(mint.supply.toNumber() === 1);
  });
});
