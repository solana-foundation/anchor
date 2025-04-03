const anchor = require("@coral-xyz/anchor");
const { assert } = require("chai");
const { BN } = require("@coral-xyz/anchor");
const {
  createAssociatedTokenAccountInstruction,
  TOKEN_PROGRAM_ID,
  mintTo,
  getAssociatedTokenAddress,
} = require("@solana/spl-token");
const {
  Keypair,
  SystemProgram,
  PublicKey,
  Transaction,
} = require("@solana/web3.js");
const {
  sleep,
  getTokenAccount,
  createTokenMint,
  createTokenAccount,
} = require("./utils");

describe("ido-pool", () => {
  const provider = anchor.AnchorProvider.local();

  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  const program = anchor.workspace.IdoPool;
  const connection = provider.connection;
  const payer = provider.wallet.payer;

  // All mints default to 6 decimal places.
  const watermelonIdoAmount = 5_000_000;

  // These are all of the variables we assume exist in the world already and
  // are available to the client.

  let usdcMint = null;
  let watermelonMint = null;
  let idoAuthorityUsdc = null;
  let idoAuthorityWatermelon = null;

  it("Initializes the state-of-the-world", async () => {
    usdcMint = await createTokenMint(provider);
    watermelonMint = await createTokenMint(provider);

    idoAuthorityUsdc = await createTokenAccount(
      provider,
      usdcMint,
      provider.wallet.publicKey
    );
    idoAuthorityWatermelon = await createTokenAccount(
      provider,
      watermelonMint,
      provider.wallet.publicKey
    );

    await mintTo(
      connection,
      payer,
      watermelonMint,
      idoAuthorityWatermelon,
      payer,
      watermelonIdoAmount
    );
    idoAuthority_watermelon_account = await getTokenAccount(
      provider,
      idoAuthorityWatermelon
    );
    assert.equal(
      idoAuthority_watermelon_account.amount,
      BigInt(watermelonIdoAmount)
    );
  });

  // These are all variables the client will need to create in order to
  // initialize the IDO pool
  let idoTimes;
  let idoName = "test_ido";

  it("Initializes the IDO pool", async () => {
    let bumps = new PoolBumps();

    const [idoAccount, idoAccountBump] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName)],
      program.programId
    );
    bumps.idoAccount = idoAccountBump;

    const [redeemableMint, redeemableMintBump] =
      PublicKey.findProgramAddressSync(
        [Buffer.from(idoName), Buffer.from("redeemable_mint")],
        program.programId
      );
    bumps.redeemableMint = redeemableMintBump;

    const [poolWatermelon, poolWatermelonBump] =
      PublicKey.findProgramAddressSync(
        [Buffer.from(idoName), Buffer.from("pool_watermelon")],
        program.programId
      );
    bumps.poolWatermelon = poolWatermelonBump;

    const [poolUsdc, poolUsdcBump] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName), Buffer.from("pool_usdc")],
      program.programId
    );
    bumps.poolUsdc = poolUsdcBump;

    idoTimes = new IdoTimes();
    const nowBn = new BN(Date.now() / 1000);
    idoTimes.startIdo = nowBn.add(new BN(5));
    idoTimes.endDeposits = nowBn.add(new BN(10));
    idoTimes.endIdo = nowBn.add(new BN(15));
    idoTimes.endEscrow = nowBn.add(new BN(16));

    await program.rpc.initializePool(
      idoName,
      bumps,
      new BN(watermelonIdoAmount),
      idoTimes,
      {
        accounts: {
          idoAuthority: provider.wallet.publicKey,
          idoAuthorityWatermelon,
          idoAccount,
          watermelonMint,
          usdcMint,
          redeemableMint,
          poolWatermelon,
          poolUsdc,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
      }
    );

    idoAuthorityWatermelonAccount = await getTokenAccount(
      provider,
      idoAuthorityWatermelon
    );
    assert.equal(idoAuthorityWatermelonAccount.amount, BigInt(0));
  });

  // We're going to need to start using the associated program account for creating token accounts
  // if not in testing, then definitely in production.

  let userUsdc = null;
  // 10 usdc
  const firstDeposit = 10_000_349;

  it("Exchanges user USDC for redeemable tokens", async () => {
    // Wait until the IDO has opened.
    if (Date.now() < idoTimes.startIdo.toNumber() * 1000) {
      await sleep(idoTimes.startIdo.toNumber() * 1000 - Date.now() + 2000);
    }

    const [idoAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName)],
      program.programId
    );

    const [redeemableMint] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName), Buffer.from("redeemable_mint")],
      program.programId
    );

    const [poolUsdc] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName), Buffer.from("pool_usdc")],
      program.programId
    );

    userUsdc = await getAssociatedTokenAddress(
      usdcMint,
      provider.wallet.publicKey
    );
    // Get the instructions to add to the RPC call
    let createUserUsdcInstr = createAssociatedTokenAccountInstruction(
      provider.wallet.publicKey,
      userUsdc,
      provider.wallet.publicKey,
      usdcMint
    );
    let createUserUsdcTrns = new Transaction().add(createUserUsdcInstr);
    await provider.sendAndConfirm(createUserUsdcTrns);

    await mintTo(connection, payer, usdcMint, userUsdc, payer, firstDeposit);

    // Check if we inited correctly
    userUsdcAccount = await getTokenAccount(provider, userUsdc);
    assert.equal(userUsdcAccount.amount, BigInt(firstDeposit));

    const [userRedeemable] = PublicKey.findProgramAddressSync(
      [
        provider.wallet.publicKey.toBuffer(),
        Buffer.from(idoName),
        Buffer.from("user_redeemable"),
      ],
      program.programId
    );

    try {
      const tx = await program.rpc.exchangeUsdcForRedeemable(
        new anchor.BN(firstDeposit),
        {
          accounts: {
            userAuthority: provider.wallet.publicKey,
            userUsdc,
            userRedeemable,
            idoAccount,
            usdcMint,
            redeemableMint,
            watermelonMint,
            poolUsdc,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          instructions: [
            program.instruction.initUserRedeemable({
              accounts: {
                userAuthority: provider.wallet.publicKey,
                userRedeemable,
                idoAccount,
                redeemableMint,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
              },
            }),
          ],
        }
      );
    } catch (err) {
      console.log("This is the error message", err.toString());
    }
    poolUsdcAccount = await getTokenAccount(provider, poolUsdc);
    assert.equal(poolUsdcAccount.amount, BigInt(firstDeposit));
    userRedeemableAccount = await getTokenAccount(provider, userRedeemable);
    assert.equal(userRedeemableAccount.amount, BigInt(firstDeposit));
  });

  // 23 usdc
  const secondDeposit = 23_000_672;
  let totalPoolUsdc, secondUserKeypair, secondUserUsdc;

  it("Exchanges a second users USDC for redeemable tokens", async () => {
    const [idoAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName)],
      program.programId
    );

    const [redeemableMint] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName), Buffer.from("redeemable_mint")],
      program.programId
    );

    const [poolUsdc] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName), Buffer.from("pool_usdc")],
      program.programId
    );

    secondUserKeypair = anchor.web3.Keypair.generate();

    transferSolInstr = SystemProgram.transfer({
      fromPubkey: provider.wallet.publicKey,
      lamports: 100_000_000_000, // 100 sol
      toPubkey: secondUserKeypair.publicKey,
    });
    secondUserUsdc = await getAssociatedTokenAddress(
      usdcMint,
      secondUserKeypair.publicKey
    );
    createSecondUserUsdcInstr = createAssociatedTokenAccountInstruction(
      provider.wallet.publicKey,
      secondUserUsdc,
      secondUserKeypair.publicKey,
      usdcMint
    );
    let createSecondUserUsdcTrns = new Transaction();
    createSecondUserUsdcTrns.add(transferSolInstr);
    createSecondUserUsdcTrns.add(createSecondUserUsdcInstr);
    await provider.sendAndConfirm(createSecondUserUsdcTrns);
    await mintTo(
      connection,
      payer,
      usdcMint,
      secondUserUsdc,
      payer,
      secondDeposit
    );

    // Checking the transfer went through
    secondUserUsdcAccount = await getTokenAccount(provider, secondUserUsdc);
    assert.equal(secondUserUsdcAccount.amount, BigInt(secondDeposit));

    const [secondUserRedeemable] = PublicKey.findProgramAddressSync(
      [
        secondUserKeypair.publicKey.toBuffer(),
        Buffer.from(idoName),
        Buffer.from("user_redeemable"),
      ],
      program.programId
    );

    await program.rpc.exchangeUsdcForRedeemable(new BN(secondDeposit), {
      accounts: {
        userAuthority: secondUserKeypair.publicKey,
        userUsdc: secondUserUsdc,
        userRedeemable: secondUserRedeemable,
        idoAccount,
        usdcMint,
        redeemableMint,
        watermelonMint,
        poolUsdc,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
      instructions: [
        program.instruction.initUserRedeemable({
          accounts: {
            userAuthority: secondUserKeypair.publicKey,
            userRedeemable: secondUserRedeemable,
            idoAccount,
            redeemableMint,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
        }),
      ],
      signers: [secondUserKeypair],
    });

    secondUserRedeemableAccount = await getTokenAccount(
      provider,
      secondUserRedeemable
    );
    assert.isTrue(secondUserRedeemableAccount.amount === BigInt(secondDeposit));

    totalPoolUsdc = BigInt(firstDeposit) + BigInt(secondDeposit);
    poolUsdcAccount = await getTokenAccount(provider, poolUsdc);
    assert.equal(poolUsdcAccount.amount, totalPoolUsdc);
  });

  const firstWithdrawal = 2_000_000;

  it("Exchanges user Redeemable tokens for USDC", async () => {
    const [idoAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName)],
      program.programId
    );

    const [redeemableMint] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName), Buffer.from("redeemable_mint")],
      program.programId
    );

    const [poolUsdc] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName), Buffer.from("pool_usdc")],
      program.programId
    );

    const [userRedeemable] = PublicKey.findProgramAddressSync(
      [
        provider.wallet.publicKey.toBuffer(),
        Buffer.from(idoName),
        Buffer.from("user_redeemable"),
      ],
      program.programId
    );

    const [escrowUsdc] = PublicKey.findProgramAddressSync(
      [
        provider.wallet.publicKey.toBuffer(),
        Buffer.from(idoName),
        Buffer.from("escrow_usdc"),
      ],
      program.programId
    );

    await program.rpc.exchangeRedeemableForUsdc(new BN(firstWithdrawal), {
      accounts: {
        userAuthority: provider.wallet.publicKey,
        escrowUsdc,
        userRedeemable,
        idoAccount,
        usdcMint,
        redeemableMint,
        watermelonMint,
        poolUsdc,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
      instructions: [
        program.instruction.initEscrowUsdc({
          accounts: {
            userAuthority: provider.wallet.publicKey,
            escrowUsdc,
            idoAccount,
            usdcMint,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
        }),
      ],
    });

    totalPoolUsdc = totalPoolUsdc - BigInt(firstWithdrawal);
    poolUsdcAccount = await getTokenAccount(provider, poolUsdc);
    assert.equal(poolUsdcAccount.amount, totalPoolUsdc);
    escrowUsdcAccount = await getTokenAccount(provider, escrowUsdc);
    assert.equal(escrowUsdcAccount.amount, BigInt(firstWithdrawal));
  });

  it("Exchanges user Redeemable tokens for watermelon", async () => {
    // Wait until the IDO has ended.
    if (Date.now() < idoTimes.endIdo.toNumber() * 1000) {
      await sleep(idoTimes.endIdo.toNumber() * 1000 - Date.now() + 3000);
    }

    const [idoAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName)],
      program.programId
    );

    const [poolWatermelon] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName), Buffer.from("pool_watermelon")],
      program.programId
    );

    const [redeemableMint] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName), Buffer.from("redeemable_mint")],
      program.programId
    );

    const [userRedeemable] = PublicKey.findProgramAddressSync(
      [
        provider.wallet.publicKey.toBuffer(),
        Buffer.from(idoName),
        Buffer.from("user_redeemable"),
      ],
      program.programId
    );

    let firstUserRedeemable = BigInt(firstDeposit) - BigInt(firstWithdrawal);
    // TODO we've been lazy here and not used an ATA as we did with USDC
    userWatermelon = await createTokenAccount(
      provider,
      watermelonMint,
      provider.wallet.publicKey
    );

    await program.rpc.exchangeRedeemableForWatermelon(
      new BN(firstUserRedeemable.valueOf()),
      {
        accounts: {
          payer: provider.wallet.publicKey,
          userAuthority: provider.wallet.publicKey,
          userWatermelon,
          userRedeemable,
          idoAccount,
          watermelonMint,
          redeemableMint,
          poolWatermelon,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
      }
    );

    poolWatermelonAccount = await getTokenAccount(provider, poolWatermelon);
    let redeemedWatermelon =
      (firstUserRedeemable * BigInt(watermelonIdoAmount)) / totalPoolUsdc;
    let remainingWatermelon = BigInt(watermelonIdoAmount) - redeemedWatermelon;
    assert.equal(poolWatermelonAccount.amount, remainingWatermelon);
    userWatermelonAccount = await getTokenAccount(provider, userWatermelon);
    assert.equal(userWatermelonAccount.amount, redeemedWatermelon);
  });

  it("Exchanges second user's Redeemable tokens for watermelon", async () => {
    const [idoAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName)],
      program.programId
    );

    const [redeemableMint] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName), Buffer.from("redeemable_mint")],
      program.programId
    );

    const [secondUserRedeemable] = PublicKey.findProgramAddressSync(
      [
        secondUserKeypair.publicKey.toBuffer(),
        Buffer.from(idoName),
        Buffer.from("user_redeemable"),
      ],
      program.programId
    );

    const [poolWatermelon] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName), Buffer.from("pool_watermelon")],
      program.programId
    );

    secondUserWatermelon = await createTokenAccount(
      provider,
      watermelonMint,
      secondUserKeypair.publicKey
    );

    await program.rpc.exchangeRedeemableForWatermelon(new BN(secondDeposit), {
      accounts: {
        payer: provider.wallet.publicKey,
        userAuthority: secondUserKeypair.publicKey,
        userWatermelon: secondUserWatermelon,
        userRedeemable: secondUserRedeemable,
        idoAccount,
        watermelonMint,
        redeemableMint,
        poolWatermelon,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });

    poolWatermelonAccount = await getTokenAccount(provider, poolWatermelon);
    assert.isTrue(poolWatermelonAccount.amount === BigInt(0));
  });

  it("Withdraws total USDC from pool account", async () => {
    const [idoAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName)],
      program.programId
    );

    const [poolUsdc] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName), Buffer.from("pool_usdc")],
      program.programId
    );

    await program.rpc.withdrawPoolUsdc({
      accounts: {
        idoAuthority: provider.wallet.publicKey,
        idoAuthorityUsdc,
        idoAccount,
        usdcMint,
        watermelonMint,
        poolUsdc,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });

    poolUsdcAccount = await getTokenAccount(provider, poolUsdc);
    assert.isTrue(poolUsdcAccount.amount === BigInt(0));
    idoAuthorityUsdcAccount = await getTokenAccount(provider, idoAuthorityUsdc);
    assert.isTrue(idoAuthorityUsdcAccount.amount === BigInt(totalPoolUsdc));
  });

  it("Withdraws USDC from the escrow account after waiting period is over", async () => {
    // Wait until the escrow period is over.
    if (Date.now() < idoTimes.endEscrow.toNumber() * 1000 + 1000) {
      await sleep(idoTimes.endEscrow.toNumber() * 1000 - Date.now() + 4000);
    }

    const [idoAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from(idoName)],
      program.programId
    );

    const [escrowUsdc] = PublicKey.findProgramAddressSync(
      [
        provider.wallet.publicKey.toBuffer(),
        Buffer.from(idoName),
        Buffer.from("escrow_usdc"),
      ],
      program.programId
    );

    await program.rpc.withdrawFromEscrow(new BN(firstWithdrawal), {
      accounts: {
        payer: provider.wallet.publicKey,
        userAuthority: provider.wallet.publicKey,
        userUsdc,
        escrowUsdc,
        idoAccount,
        usdcMint,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });

    userUsdcAccount = await getTokenAccount(provider, userUsdc);
    assert.equal(userUsdcAccount.amount, BigInt(firstWithdrawal));
  });

  function PoolBumps() {
    this.idoAccount;
    this.redeemableMint;
    this.poolWatermelon;
    this.poolUsdc;
  }

  function IdoTimes() {
    this.startIdo;
    this.endDeposts;
    this.endIdo;
    this.endEscrow;
  }
});
