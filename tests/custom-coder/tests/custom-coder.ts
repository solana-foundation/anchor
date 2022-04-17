import * as anchor from "@project-serum/anchor";
import { Spl } from "@project-serum/anchor";
import { assert } from "chai";
import BN from "bn.js";
import { Keypair, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";

describe("custom-coder", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Client.
  const program = Spl.token();

  // Constants.
  const mintKeypair = Keypair.generate();
  const aliceTokenKeypair = Keypair.generate();
  const bobTokenKeypair = Keypair.generate();
  const rent = SYSVAR_RENT_PUBKEY;

  it("Creates a mint", async () => {
    await program.rpc.initializeMint(6, provider.wallet.publicKey, null, {
      accounts: {
        mint: mintKeypair.publicKey,
        rent,
      },
      signers: [mintKeypair],
      preInstructions: [
        await program.account.mint.createInstruction(mintKeypair),
      ],
    });
    const mintAccount = await program.account.mint.fetch(mintKeypair.publicKey);
    assert.isTrue(mintAccount.mintAuthority.equals(provider.wallet.publicKey));
    assert.isNull(mintAccount.freezeAuthority);
    assert.strictEqual(mintAccount.decimals, 6);
    assert.isTrue(mintAccount.isInitialized);
    assert.strictEqual(mintAccount.supply.toNumber(), 0);
  });

  it("Creates a token account for alice", async () => {
    await program.rpc.initializeAccount({
      accounts: {
        account: aliceTokenKeypair.publicKey,
        mint: mintKeypair.publicKey,
        authority: provider.wallet.publicKey,
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
    assert.isTrue(token.authority.equals(provider.wallet.publicKey));
    assert.isTrue(token.mint.equals(mintKeypair.publicKey));
    assert.strictEqual(token.amount.toNumber(), 0);
    assert.isNull(token.delegate);
    assert.strictEqual(token.state, 1);
    assert.isNull(token.isNative);
    assert.strictEqual(token.delegatedAmount.toNumber(), 0);
    assert.isNull(token.closeAuthority);
  });

  it("Mints a token to alice", async () => {
    await program.rpc.mintTo(new BN(2), {
      accounts: {
        mint: mintKeypair.publicKey,
        to: aliceTokenKeypair.publicKey,
        authority: provider.wallet.publicKey,
      },
    });

    const token = await program.account.token.fetch(
      aliceTokenKeypair.publicKey
    );
    const mint = await program.account.mint.fetch(mintKeypair.publicKey);
    assert.strictEqual(token.amount.toNumber(), 2);
    assert.strictEqual(mint.supply.toNumber(), 2);
  });

  it("Creates a token for bob", async () => {
    await program.rpc.initializeAccount({
      accounts: {
        account: bobTokenKeypair.publicKey,
        mint: mintKeypair.publicKey,
        authority: provider.wallet.publicKey,
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
        authority: provider.wallet.publicKey,
      },
    });
    const aliceToken = await program.account.token.fetch(
      aliceTokenKeypair.publicKey
    );
    const bobToken = await program.account.token.fetch(
      bobTokenKeypair.publicKey
    );
    assert.strictEqual(aliceToken.amount.toNumber(), 1);
    assert.strictEqual(bobToken.amount.toNumber(), 1);
  });

  it("Alice burns a token", async () => {
    await program.rpc.burn(new BN(1), {
      accounts: {
        source: aliceTokenKeypair.publicKey,
        mint: mintKeypair.publicKey,
        authority: provider.wallet.publicKey,
      },
    });
    const aliceToken = await program.account.token.fetch(
      aliceTokenKeypair.publicKey
    );
    const mint = await program.account.mint.fetch(mintKeypair.publicKey);
    assert.strictEqual(aliceToken.amount.toNumber(), 0);
    assert.strictEqual(mint.supply.toNumber(), 1);
  });
});
