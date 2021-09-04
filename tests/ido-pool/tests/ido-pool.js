const anchor = require("@project-serum/anchor");
const assert = require("assert");
const {
  TOKEN_PROGRAM_ID,
  sleep,
  getTokenAccount,
  createMint,
  createTokenAccount,
} = require("./utils");

describe("ido-pool", () => {
  const provider = anchor.Provider.local();

  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  const program = anchor.workspace.IdoPool;

  // All mints default to 6 decimal places.
  const watermelonIdoAmount = new anchor.BN(5000000);

  // These are all of the variables we assume exist in the world already and
  // are available to the client.
  let usdcMintToken = null;
  let usdcMint = null;
  let watermelonMintToken = null;
  let watermelonMint = null;
  let creatorUsdc = null;
  let creatorWatermelon = null;

  it("Initializes the state-of-the-world", async () => {
    usdcMintToken = await createMint(provider);
    watermelonMintToken = await createMint(provider);
    usdcMint = usdcMintToken.publicKey;
    watermelonMint = watermelonMintToken.publicKey;
    creatorUsdc = await createTokenAccount(
      provider,
      usdcMint,
      provider.wallet.publicKey
    );
    creatorWatermelon = await createTokenAccount(
      provider,
      watermelonMint,
      provider.wallet.publicKey
    );
    // Mint Watermelon tokens the will be distributed from the IDO pool.
    await watermelonMintToken.mintTo(
      creatorWatermelon,
      provider.wallet.publicKey,
      [],
      watermelonIdoAmount.toString(),
    );
    creator_watermelon_account = await getTokenAccount(
      provider,
      creatorWatermelon
    );
    assert.ok(creator_watermelon_account.amount.eq(watermelonIdoAmount));
  });

  // These are all variables the client will have to create to initialize the
  // IDO pool
  let poolSigner = null;
  let redeemableMintToken = null;
  let redeemableMint = null;
  let poolWatermelon = null;
  let poolUsdc = null;
  let poolAccount = null;

  let startIdoTs = null;
  let endDepositsTs = null;
  let endIdoTs = null;

  it("Initializes the IDO pool", async () => {
    // We use the watermelon mint address as the seed, could use something else though.
    const [_poolSigner, nonce] = await anchor.web3.PublicKey.findProgramAddress(
      [watermelonMint.toBuffer()],
      program.programId
    );
    poolSigner = _poolSigner;

    // Pool doesn't need a Redeemable SPL token account because it only
    // burns and mints redeemable tokens, it never stores them.
    redeemableMintToken = await createMint(provider, poolSigner);
    redeemableMint = redeemableMintToken.publicKey;
    poolWatermelon = await createTokenAccount(
      provider,
      watermelonMint,
      poolSigner
    );
    poolUsdc = await createTokenAccount(provider, usdcMint, poolSigner);

    poolAccount = anchor.web3.Keypair.generate();
    const nowBn = new anchor.BN(Date.now() / 1000);
    startIdoTs = nowBn.add(new anchor.BN(5));
    endDepositsTs = nowBn.add(new anchor.BN(10));
    endIdoTs = nowBn.add(new anchor.BN(15));

    // Atomically create the new account and initialize it with the program.
    await program.rpc.initializePool(
      watermelonIdoAmount,
      nonce,
      startIdoTs,
      endDepositsTs,
      endIdoTs,
      {
        accounts: {
          poolAccount: poolAccount.publicKey,
          poolSigner,
          distributionAuthority: provider.wallet.publicKey,
          creatorWatermelon,
          redeemableMint,
          usdcMint,
          poolWatermelon,
          poolUsdc,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        },
        signers: [poolAccount],
        instructions: [
          await program.account.poolAccount.createInstruction(poolAccount),
        ],
      }
    );

    creators_watermelon_account = await getTokenAccount(
      provider,
      creatorWatermelon
    );
    assert.ok(creators_watermelon_account.amount.eq(new anchor.BN(0)));
  });

  // We're going to need to start using the associated program account for creating token accounts
  // if not in testing, then definitely in production.

  let userUsdc = null;
  let userRedeemable = null;
  // 10 usdc
  const firstDeposit = new anchor.BN(10_000_349);

  it("Exchanges user USDC for redeemable tokens", async () => {
    // Wait until the IDO has opened.
    if (Date.now() < startIdoTs.toNumber() * 1000) {
      await sleep(startIdoTs.toNumber() * 1000 - Date.now() + 1000);
    }

    userUsdc = await createTokenAccount(
      provider,
      usdcMint,
      provider.wallet.publicKey
    );
    await usdcMintToken.mintTo(
      userUsdc,
      provider.wallet.publicKey,
      [],
      firstDeposit.toString(),
    );
    userRedeemable = await createTokenAccount(
      provider,
      redeemableMint,
      provider.wallet.publicKey
    );

    try {
      const tx = await program.rpc.exchangeUsdcForRedeemable(firstDeposit, {
        accounts: {
          poolAccount: poolAccount.publicKey,
          poolSigner,
          redeemableMint,
          poolUsdc,
          userAuthority: provider.wallet.publicKey,
          userUsdc,
          userRedeemable,
          tokenProgram: TOKEN_PROGRAM_ID,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        },
      });
    } catch (err) {
      console.log("This is the error message", err.toString());
    }
    poolUsdcAccount = await getTokenAccount(provider, poolUsdc);
    assert.ok(poolUsdcAccount.amount.eq(firstDeposit));
    userRedeemableAccount = await getTokenAccount(provider, userRedeemable);
    assert.ok(userRedeemableAccount.amount.eq(firstDeposit));
  });

  // 23 usdc
  const secondDeposit = new anchor.BN(23_000_672);
  let totalPoolUsdc = null;

  it("Exchanges a second users USDC for redeemable tokens", async () => {
    secondUserUsdc = await createTokenAccount(
      provider,
      usdcMint,
      provider.wallet.publicKey
    );
    await usdcMintToken.mintTo(
      secondUserUsdc,
      provider.wallet.publicKey,
      [],
      secondDeposit.toString(),
    );
    secondUserRedeemable = await createTokenAccount(
      provider,
      redeemableMint,
      provider.wallet.publicKey
    );

    await program.rpc.exchangeUsdcForRedeemable(secondDeposit, {
      accounts: {
        poolAccount: poolAccount.publicKey,
        poolSigner,
        redeemableMint,
        poolUsdc,
        userAuthority: provider.wallet.publicKey,
        userUsdc: secondUserUsdc,
        userRedeemable: secondUserRedeemable,
        tokenProgram: TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      },
    });

    totalPoolUsdc = firstDeposit.add(secondDeposit);
    poolUsdcAccount = await getTokenAccount(provider, poolUsdc);
    assert.ok(poolUsdcAccount.amount.eq(totalPoolUsdc));
    secondUserRedeemableAccount = await getTokenAccount(
      provider,
      secondUserRedeemable
    );
    assert.ok(secondUserRedeemableAccount.amount.eq(secondDeposit));
  });

  const firstWithdrawal = new anchor.BN(2_000_000);

  it("Exchanges user Redeemable tokens for USDC", async () => {
    await program.rpc.exchangeRedeemableForUsdc(firstWithdrawal, {
      accounts: {
        poolAccount: poolAccount.publicKey,
        poolSigner,
        redeemableMint,
        poolUsdc,
        userAuthority: provider.wallet.publicKey,
        userUsdc,
        userRedeemable,
        tokenProgram: TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      },
    });

    totalPoolUsdc = totalPoolUsdc.sub(firstWithdrawal);
    poolUsdcAccount = await getTokenAccount(provider, poolUsdc);
    assert.ok(poolUsdcAccount.amount.eq(totalPoolUsdc));
    userUsdcAccount = await getTokenAccount(provider, userUsdc);
    assert.ok(userUsdcAccount.amount.eq(firstWithdrawal));
  });

  it("Exchanges user Redeemable tokens for watermelon", async () => {
    // Wait until the IDO has opened.
    if (Date.now() < endIdoTs.toNumber() * 1000) {
      await sleep(endIdoTs.toNumber() * 1000 - Date.now() + 2000);
    }
    let firstUserRedeemable = firstDeposit.sub(firstWithdrawal);
    userWatermelon = await createTokenAccount(
      provider,
      watermelonMint,
      provider.wallet.publicKey
    );

    await program.rpc.exchangeRedeemableForWatermelon(firstUserRedeemable, {
      accounts: {
        poolAccount: poolAccount.publicKey,
        poolSigner,
        redeemableMint,
        poolWatermelon,
        userAuthority: provider.wallet.publicKey,
        userWatermelon,
        userRedeemable,
        tokenProgram: TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      },
    });

    poolWatermelonAccount = await getTokenAccount(provider, poolWatermelon);
    let redeemedWatermelon = firstUserRedeemable
      .mul(watermelonIdoAmount)
      .div(totalPoolUsdc);
    let remainingWatermelon = watermelonIdoAmount.sub(redeemedWatermelon);
    assert.ok(poolWatermelonAccount.amount.eq(remainingWatermelon));
    userWatermelonAccount = await getTokenAccount(provider, userWatermelon);
    assert.ok(userWatermelonAccount.amount.eq(redeemedWatermelon));
  });

  it("Exchanges second users Redeemable tokens for watermelon", async () => {
    secondUserWatermelon = await createTokenAccount(
      provider,
      watermelonMint,
      provider.wallet.publicKey
    );

    await program.rpc.exchangeRedeemableForWatermelon(secondDeposit, {
      accounts: {
        poolAccount: poolAccount.publicKey,
        poolSigner,
        redeemableMint,
        poolWatermelon,
        userAuthority: provider.wallet.publicKey,
        userWatermelon: secondUserWatermelon,
        userRedeemable: secondUserRedeemable,
        tokenProgram: TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      },
    });

    poolWatermelonAccount = await getTokenAccount(provider, poolWatermelon);
    assert.ok(poolWatermelonAccount.amount.eq(new anchor.BN(0)));
  });

  it("Withdraws total USDC from pool account", async () => {
    await program.rpc.withdrawPoolUsdc({
      accounts: {
        poolAccount: poolAccount.publicKey,
        poolSigner,
        distributionAuthority: provider.wallet.publicKey,
        creatorUsdc,
        poolUsdc,
        tokenProgram: TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      },
    });

    poolUsdcAccount = await getTokenAccount(provider, poolUsdc);
    assert.ok(poolUsdcAccount.amount.eq(new anchor.BN(0)));
    creatorUsdcAccount = await getTokenAccount(provider, creatorUsdc);
    assert.ok(creatorUsdcAccount.amount.eq(totalPoolUsdc));
  });
});
