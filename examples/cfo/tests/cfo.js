const assert = require("assert");
const { Token } = require("@solana/spl-token");
const anchor = require("@project-serum/anchor");
const serumCmn = require("@project-serum/common");
const { Market } = require("@project-serum/serum");
const { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } = anchor.web3;
const utils = require("./utils");
const { setupStakePool } = require("./utils/stake");

const DEX_PID = new PublicKey("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");
const SWAP_PID = new PublicKey("22Y43yTVxuUkoRKdm9thyRhQ3SdgQS7c7kB6UNCiaczD");
const TOKEN_PID = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const REGISTRY_PID = new PublicKey(
  "GrAkKfEpTKQuVHG2Y97Y2FF4i7y7Q5AHLK94JBy7Y5yv"
);
const LOCKUP_PID = new PublicKey(
  "6ebQNeTPZ1j7k3TtkCCtEPRvG7GQsucQrZ7sSEDQi9Ks"
);
const FEES = "6160355581";

describe("cfo", () => {
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.Cfo;
  let officer;
  let TOKEN_CLIENT;
  let officerAccount;
  const sweepAuthority = program.provider.wallet.publicKey;

  // Accounts used to setup the orderbook.
  let ORDERBOOK_ENV,
    // Accounts used for A -> USDC swap transactions.
    SWAP_A_USDC_ACCOUNTS,
    // Accounts used for  USDC -> A swap transactions.
    SWAP_USDC_A_ACCOUNTS,
    // Serum DEX vault PDA for market A/USDC.
    marketAVaultSigner,
    // Serum DEX vault PDA for market B/USDC.
    marketBVaultSigner;

  let registrar, msrmRegistrar;

  it("BOILERPLATE: Sets up a market with funded fees", async () => {
    ORDERBOOK_ENV = await utils.initMarket({
      provider: program.provider,
    });
    console.log("Token A: ", ORDERBOOK_ENV.marketA.baseMintAddress.toString());
    console.log(
      "Token USDC: ",
      ORDERBOOK_ENV.marketA.quoteMintAddress.toString()
    );
    TOKEN_CLIENT = new Token(
      program.provider.connection,
      ORDERBOOK_ENV.usdc,
      TOKEN_PID,
      program.provider.wallet.payer
    );

    await TOKEN_CLIENT.transfer(
      ORDERBOOK_ENV.godUsdc,
      ORDERBOOK_ENV.marketA._decoded.quoteVault,
      program.provider.wallet.payer,
      [],
      10000000000000
    );

    const tokenAccount = await TOKEN_CLIENT.getAccountInfo(
      ORDERBOOK_ENV.marketA._decoded.quoteVault
    );
    assert.ok(tokenAccount.amount.toString() === "10000902263700");
  });

  it("BOILERPLATE: Executes trades to generate fees", async () => {
    await utils.runTradeBot(
      ORDERBOOK_ENV.marketA._decoded.ownAddress,
      program.provider,
      1
    );
    let marketClient = await Market.load(
      program.provider.connection,
      ORDERBOOK_ENV.marketA._decoded.ownAddress,
      { commitment: "recent" },
      DEX_PID
    );
    assert.ok(marketClient._decoded.quoteFeesAccrued.toString() === FEES);
  });

  it("BOILERPLATE: Sets up the staking pools", async () => {
    await setupStakePool(ORDERBOOK_ENV.mintA, ORDERBOOK_ENV.godA);
    registrar = ORDERBOOK_ENV.usdc;
    msrmRegistrar = registrar;
  });

  it("Creates a CFO!", async () => {
    let distribution = {
      burn: 80,
      stake: 20,
      treasury: 0,
    };
    const [_officer, officerBump] = await PublicKey.findProgramAddress(
      [DEX_PID.toBuffer()],
      program.programId
    );
    officer = _officer;
    const [srmVault, srmBump] = await PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("vault"), officer.toBuffer()],
      program.programId
    );
    const [stake, stakeBump] = await PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("stake"), officer.toBuffer()],
      program.programId
    );
    const [treasury, treasuryBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("treasury")),
        officer.toBuffer(),
      ],
      program.programId
    );
    const bumps = {
      bump: officerBump,
      srm: srmBump,
      stake: stakeBump,
      treasury: treasuryBump,
    };
    await program.rpc.createOfficer(
      bumps,
      distribution,
      registrar,
      msrmRegistrar,
      {
        accounts: {
          officer,
          srmVault,
          stake,
          treasury,
          mint: ORDERBOOK_ENV.mintA,
          authority: program.provider.wallet.publicKey,
          dexProgram: DEX_PID,
          swapProgram: SWAP_PID,
          tokenProgram: TOKEN_PID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        },
      }
    );

    officerAccount = await program.account.officer.fetch(officer);
    assert.ok(
      officerAccount.authority.equals(program.provider.wallet.publicKey)
    );
    assert.ok(
      JSON.stringify(officerAccount.distribution) ===
        JSON.stringify(distribution)
    );
  });

  it("Creates a token account for the officer associated with the market", async () => {
    const [token, bump] = await PublicKey.findProgramAddress(
      [officer.toBuffer(), ORDERBOOK_ENV.usdc.toBuffer()],
      program.programId
    );
    await program.rpc.createOfficerToken(bump, {
      accounts: {
        officer,
        token,
        mint: ORDERBOOK_ENV.usdc,
        payer: program.provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PID,
        rent: SYSVAR_RENT_PUBKEY,
      },
    });
    const tokenAccount = await TOKEN_CLIENT.getAccountInfo(token);
    assert.ok(tokenAccount.state === 1);
    assert.ok(tokenAccount.isInitialized);
  });

  it("Sweeps fees", async () => {
    const [sweepVault, bump] = await PublicKey.findProgramAddress(
      [officer.toBuffer(), ORDERBOOK_ENV.usdc.toBuffer()],
      program.programId
    );
    const beforeTokenAccount = await serumCmn.getTokenAccount(
      program.provider,
      sweepVault
    );
    await program.rpc.sweepFees({
      accounts: {
        officer,
        sweepVault,
        mint: ORDERBOOK_ENV.usdc,
        dex: {
          market: ORDERBOOK_ENV.marketA._decoded.ownAddress,
          pcVault: ORDERBOOK_ENV.marketA._decoded.quoteVault,
          sweepAuthority,
          vaultSigner: ORDERBOOK_ENV.vaultSigner,
          dexProgram: DEX_PID,
          tokenProgram: TOKEN_PID,
        },
      },
    });
    const afterTokenAccount = await serumCmn.getTokenAccount(
      program.provider,
      sweepVault
    );
    assert.ok(
      afterTokenAccount.amount.sub(beforeTokenAccount.amount).toString() ===
        FEES
    );
  });

  it("TODO", async () => {
    // todo
  });
});
