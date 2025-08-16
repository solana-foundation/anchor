const { assert } = require("chai");
const anchor = require("@coral-xyz/anchor");
const { BN } = anchor;
const { OpenOrders } = require("@project-serum/serum");
const { TOKEN_PROGRAM_ID } = require("@solana/spl-token");
const serumCmn = require("@project-serum/common");
const utils = require("./utils");

// Taker fee rate (bps).
const TAKER_FEE = 0.0022;

describe("swap", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  provider.send = provider.sendAndConfirm;
  anchor.setProvider(provider);

  // Swap program client.
  const program = anchor.workspace.Swap;

  // Accounts and environment variables.
  let ORDERBOOK_ENV,
    SWAP_A_USDC_ACCOUNTS,
    SWAP_USDC_A_ACCOUNTS,
    marketAVaultSigner,
    marketBVaultSigner;

  const openOrdersA = anchor.web3.Keypair.generate();
  const openOrdersB = anchor.web3.Keypair.generate();

  it("BOILERPLATE: Sets up two markets with resting orders", async () => {
    ORDERBOOK_ENV = await utils.setupTwoMarkets({
      provider: program.provider,
    });
  });

  it("BOILERPLATE: Sets up reusable accounts", async () => {
    const { marketA, marketB } = ORDERBOOK_ENV;

    [marketAVaultSigner] = await utils.getVaultOwnerAndNonce(
      marketA._decoded.ownAddress
    );
    [marketBVaultSigner] = await utils.getVaultOwnerAndNonce(
      marketB._decoded.ownAddress
    );

    const commonAccounts = {
      pcWallet: ORDERBOOK_ENV.godUsdc,
      authority: program.provider.wallet.publicKey,
      dexProgram: utils.DEX_PID,
      tokenProgram: TOKEN_PROGRAM_ID,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    };

    SWAP_USDC_A_ACCOUNTS = {
      ...commonAccounts,
      market: {
        market: marketA._decoded.ownAddress,
        requestQueue: marketA._decoded.requestQueue,
        eventQueue: marketA._decoded.eventQueue,
        bids: marketA._decoded.bids,
        asks: marketA._decoded.asks,
        coinVault: marketA._decoded.baseVault,
        pcVault: marketA._decoded.quoteVault,
        vaultSigner: marketAVaultSigner,
        openOrders: openOrdersA.publicKey,
        orderPayerTokenAccount: ORDERBOOK_ENV.godUsdc,
        coinWallet: ORDERBOOK_ENV.godA,
      },
    };

    SWAP_A_USDC_ACCOUNTS = {
      ...commonAccounts,
      market: {
        ...SWAP_USDC_A_ACCOUNTS.market,
        orderPayerTokenAccount: ORDERBOOK_ENV.godA,
        coinWallet: ORDERBOOK_ENV.godA,
      },
    };
  });

  it("Swaps from USDC to Token A", async () => {
    const expectedResultantAmount = 1.2;
    const bestOfferPrice = 6.041;
    const amountToSpend = expectedResultantAmount * bestOfferPrice;
    const swapAmount = new BN((amountToSpend / (1 - TAKER_FEE)) * 10 ** 6);

    const [tokenAChange, usdcChange] = await withBalanceChange(
      program.provider,
      [ORDERBOOK_ENV.godA, ORDERBOOK_ENV.godUsdc],
      async () => {
        await program.rpc.swap(Side.Bid, swapAmount, new BN(1.0), {
          accounts: SWAP_USDC_A_ACCOUNTS,
          instructions: [
            await OpenOrders.makeCreateAccountTransaction(
              program.provider.connection,
              ORDERBOOK_ENV.marketA._decoded.ownAddress,
              program.provider.wallet.publicKey,
              openOrdersA.publicKey,
              utils.DEX_PID
            ),
            await OpenOrders.makeCreateAccountTransaction(
              program.provider.connection,
              ORDERBOOK_ENV.marketB._decoded.ownAddress,
              program.provider.wallet.publicKey,
              openOrdersB.publicKey,
              utils.DEX_PID
            ),
          ],
          signers: [openOrdersA, openOrdersB],
        });
      }
    );

    assert.strictEqual(tokenAChange, expectedResultantAmount);
    assert.strictEqual(usdcChange, -swapAmount.toNumber() / 10 ** 6);
  });

  it("Swaps from Token A to USDC", async () => {
    const swapAmount = 8.1;
    const bestBidPrice = 6.004;
    const amountToFill = swapAmount * bestBidPrice;
    const resultantAmount = new BN(amountToFill * (1 - TAKER_FEE) * 10 ** 6);

    const [tokenAChange, usdcChange] = await withBalanceChange(
      program.provider,
      [ORDERBOOK_ENV.godA, ORDERBOOK_ENV.godUsdc],
      async () => {
        await program.rpc.swap(
          Side.Ask,
          new BN(swapAmount * 10 ** 6),
          new BN(swapAmount),
          {
            accounts: SWAP_A_USDC_ACCOUNTS,
          }
        );
      }
    );

    assert.strictEqual(tokenAChange, -swapAmount);
    assert.strictEqual(usdcChange, resultantAmount.toNumber() / 10 ** 6);
  });

  it("Swaps from Token A to Token B", async () => {
    const swapAmount = 10;

    const [tokenAChange, tokenBChange, usdcChange] = await withBalanceChange(
      program.provider,
      [ORDERBOOK_ENV.godA, ORDERBOOK_ENV.godB, ORDERBOOK_ENV.godUsdc],
      async () => {
        await program.rpc.swapTransitive(
          new BN(swapAmount * 10 ** 6),
          new BN(swapAmount - 1),
          {
            accounts: {
              from: SWAP_A_USDC_ACCOUNTS.market,
              to: SWAP_USDC_A_ACCOUNTS.market,
              pcWallet: ORDERBOOK_ENV.godUsdc,
              authority: program.provider.wallet.publicKey,
              dexProgram: utils.DEX_PID,
              tokenProgram: TOKEN_PROGRAM_ID,
              rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            },
          }
        );
      }
    );

    assert.strictEqual(tokenAChange, -swapAmount);
    assert.strictEqual(tokenBChange, 9.8);
    assert.strictEqual(usdcChange, 0);
  });

  it("Swaps from Token B to Token A", async () => {
    const swapAmount = 23;

    const [tokenAChange, tokenBChange, usdcChange] = await withBalanceChange(
      program.provider,
      [ORDERBOOK_ENV.godA, ORDERBOOK_ENV.godB, ORDERBOOK_ENV.godUsdc],
      async () => {
        await program.rpc.swapTransitive(
          new BN(swapAmount * 10 ** 6),
          new BN(swapAmount - 1),
          {
            accounts: {
              from: SWAP_USDC_A_ACCOUNTS.market,
              to: SWAP_A_USDC_ACCOUNTS.market,
              pcWallet: ORDERBOOK_ENV.godUsdc,
              authority: program.provider.wallet.publicKey,
              dexProgram: utils.DEX_PID,
              tokenProgram: TOKEN_PROGRAM_ID,
              rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            },
          }
        );
      }
    );

    assert.strictEqual(tokenAChange, 22.6);
    assert.strictEqual(tokenBChange, -swapAmount);
    assert.strictEqual(usdcChange, 0);
  });
});

// Side rust enum used for the program's RPC API.
const Side = {
  Bid: { bid: {} },
  Ask: { ask: {} },
};

// Executes a closure, returning the change in balances before and after execution.
async function withBalanceChange(provider, accounts, fn) {
  const beforeBalances = await Promise.all(
    accounts.map(async (account) =>
      (await serumCmn.getTokenAccount(provider, account)).amount
    )
  );

  await fn();

  const afterBalances = await Promise.all(
    accounts.map(async (account) =>
      (await serumCmn.getTokenAccount(provider, account)).amount
    )
  );

  return afterBalances.map(
    (after, idx) => (after.toNumber() - beforeBalances[idx].toNumber()) / 10 ** 6
  );
}
