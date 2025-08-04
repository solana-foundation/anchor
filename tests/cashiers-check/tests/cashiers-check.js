const anchor = require("@coral-xyz/anchor");
const { Keypair, SystemProgram, PublicKey } = require("@solana/web3.js");
const { assert } = require("chai");
const {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
  createInitializeAccountInstruction,
} = require("@solana/spl-token");
const TOKEN_ACCOUNT_SIZE = 165;

describe("cashiers-check", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  provider.send = provider.sendAndConfirm;
  anchor.setProvider(provider);

  const program = anchor.workspace.CashiersCheck;
  const connection = provider.connection;
  const payer = provider.wallet.payer;
  const walletKey = provider.wallet.publicKey;

  let mint = null;
  let god = null;
  let receiver = null;

  it("Sets up initial test state", async () => {
    mint = await createMint(connection, payer, walletKey, walletKey, 6);

    god = await createAccount(
      connection,
      payer,
      mint,
      walletKey,
      Keypair.generate()
    );
    // Mint tokens to god account
    await mintTo(connection, payer, mint, god, payer, 1_000_000);

    receiver = await createAccount(
      connection,
      payer,
      mint,
      walletKey,
      Keypair.generate()
    );
  });

  const check = anchor.web3.Keypair.generate();
  const vault = anchor.web3.Keypair.generate();

  let checkSigner = null;

  it("Creates a check!", async () => {
    let [_checkSigner, nonce] = PublicKey.findProgramAddressSync(
      [check.publicKey.toBuffer()],
      program.programId
    );
    checkSigner = _checkSigner;
    const token_lamports = await connection.getMinimumBalanceForRentExemption(
      TOKEN_ACCOUNT_SIZE
    );

    await program.rpc.createCheck(new anchor.BN(100), "Hello world", nonce, {
      accounts: {
        check: check.publicKey,
        vault: vault.publicKey,
        checkSigner,
        from: god,
        to: receiver,
        owner: walletKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [check, vault],
      instructions: [
        await program.account.check.createInstruction(
          check,
          TOKEN_ACCOUNT_SIZE
        ),
        SystemProgram.createAccount({
          fromPubkey: walletKey,
          newAccountPubkey: vault.publicKey,
          space: TOKEN_ACCOUNT_SIZE,
          programId: TOKEN_PROGRAM_ID,
          lamports: token_lamports,
        }),

        createInitializeAccountInstruction(
          vault.publicKey, // account
          mint, // mint
          checkSigner // owner
        ),
      ],
    });

    const checkAccount = await program.account.check.fetch(check.publicKey);
    assert.isTrue(checkAccount.from.equals(god));
    assert.isTrue(checkAccount.to.equals(receiver));
    assert.isTrue(checkAccount.amount.eq(new anchor.BN(100)));
    assert.strictEqual(checkAccount.memo, "Hello world");
    assert.isTrue(checkAccount.vault.equals(vault.publicKey));
    assert.strictEqual(checkAccount.nonce, nonce);
    assert.isFalse(checkAccount.burned);

    let vaultAccount = await getAccount(connection, checkAccount.vault);
    assert.equal(vaultAccount.amount, BigInt(100));
  });

  it("Cashes a check", async () => {
    await program.rpc.cashCheck({
      accounts: {
        check: check.publicKey,
        vault: vault.publicKey,
        checkSigner: checkSigner,
        to: receiver,
        owner: program.provider.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });

    const checkAccount = await program.account.check.fetch(check.publicKey);
    assert.isTrue(checkAccount.burned);

    let vaultAccount = await getAccount(connection, checkAccount.vault);
    assert.equal(vaultAccount.amount, BigInt(0));

    let receiverAccount = await getAccount(connection, receiver);
    assert.equal(receiverAccount.amount, BigInt(100));
  });
});
