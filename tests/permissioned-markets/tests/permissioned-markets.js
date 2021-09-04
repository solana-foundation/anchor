const assert = require("assert");
const { Token, TOKEN_PROGRAM_ID } = require("@solana/spl-token");
const anchor = require("@project-serum/anchor");
const serum = require("@project-serum/serum");
const { BN } = anchor;
const {
  Keypair,
  Transaction,
  TransactionInstruction,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} = anchor.web3;
const {
  DexInstructions,
  OpenOrders,
  OpenOrdersPda,
  Logger,
  ReferralFees,
  MarketProxyBuilder,
} = serum;
const { initMarket, sleep } = require("./utils");

const DEX_PID = new PublicKey("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");
const REFERRAL_AUTHORITY = new PublicKey(
  "3oSfkjQZKCneYvsCTZc9HViGAPqR8pYr4h9YeGB5ZxHf"
);

describe("permissioned-markets", () => {
  // Anchor client setup.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);
  const programs = [
    anchor.workspace.PermissionedMarkets,
    anchor.workspace.PermissionedMarketsMiddleware,
  ];

  programs.forEach((program, index) => {
    // Token client.
    let usdcClient;

    // Global DEX accounts and clients shared across all tests.
    let marketProxy, tokenAccount, usdcAccount;
    let openOrders, openOrdersBump, openOrdersInitAuthority, openOrdersBumpinit;
    let usdcPosted;
    let referralTokenAddress;

    it("BOILERPLATE: Initializes an orderbook", async () => {
      const getAuthority = async (market) => {
        return (
          await PublicKey.findProgramAddress(
            [
              anchor.utils.bytes.utf8.encode("open-orders-init"),
              DEX_PID.toBuffer(),
              market.toBuffer(),
            ],
            program.programId
          )
        )[0];
      };
      const marketLoader = async (market) => {
        return new MarketProxyBuilder()
          .middleware(
            new OpenOrdersPda({
              proxyProgramId: program.programId,
              dexProgramId: DEX_PID,
            })
          )
          .middleware(new ReferralFees())
          .middleware(new Identity())
          .middleware(new Logger())
          .load({
            connection: provider.connection,
            market,
            dexProgramId: DEX_PID,
            proxyProgramId: program.programId,
            options: { commitment: "recent" },
          });
      };
      const { marketA, godA, godUsdc, usdc } = await initMarket({
        provider,
        getAuthority,
        proxyProgramId: program.programId,
        marketLoader,
      });
      marketProxy = marketA;
      usdcAccount = godUsdc;
      tokenAccount = godA;

      usdcClient = new Token(
        provider.connection,
        usdc,
        TOKEN_PROGRAM_ID,
        provider.wallet.payer
      );

      referral = await usdcClient.createAccount(REFERRAL_AUTHORITY);
    });

    it("BOILERPLATE: Calculates open orders addresses", async () => {
      const [_openOrders, bump] = await PublicKey.findProgramAddress(
        [
          anchor.utils.bytes.utf8.encode("open-orders"),
          DEX_PID.toBuffer(),
          marketProxy.market.address.toBuffer(),
          program.provider.wallet.publicKey.toBuffer(),
        ],
        program.programId
      );
      const [
        _openOrdersInitAuthority,
        bumpInit,
      ] = await PublicKey.findProgramAddress(
        [
          anchor.utils.bytes.utf8.encode("open-orders-init"),
          DEX_PID.toBuffer(),
          marketProxy.market.address.toBuffer(),
        ],
        program.programId
      );

      // Save global variables re-used across tests.
      openOrders = _openOrders;
      openOrdersBump = bump;
      openOrdersInitAuthority = _openOrdersInitAuthority;
      openOrdersBumpInit = bumpInit;
    });

    it("Creates an open orders account", async () => {
      const tx = new Transaction();
      tx.add(
        await marketProxy.instruction.initOpenOrders(
          program.provider.wallet.publicKey,
          marketProxy.market.address,
          marketProxy.market.address, // Dummy. Replaced by middleware.
          marketProxy.market.address // Dummy. Replaced by middleware.
        )
      );
      await provider.send(tx);

      const account = await provider.connection.getAccountInfo(openOrders);
      assert.ok(account.owner.toString() === DEX_PID.toString());
    });

    it("Posts a bid on the orderbook", async () => {
      const size = 1;
      const price = 1;
      usdcPosted = new BN(
        marketProxy.market._decoded.quoteLotSize.toNumber()
      ).mul(
        marketProxy.market
          .baseSizeNumberToLots(size)
          .mul(marketProxy.market.priceNumberToLots(price))
      );

      const tx = new Transaction();
      tx.add(
        marketProxy.instruction.newOrderV3({
          owner: program.provider.wallet.publicKey,
          payer: usdcAccount,
          side: "buy",
          price,
          size,
          orderType: "postOnly",
          clientId: new BN(999),
          openOrdersAddressKey: openOrders,
          selfTradeBehavior: "abortTransaction",
        })
      );
      await provider.send(tx);
    });

    it("Cancels a bid on the orderbook", async () => {
      // Given.
      const beforeOoAccount = await OpenOrders.load(
        provider.connection,
        openOrders,
        DEX_PID
      );

      // When.
      const tx = new Transaction();
      tx.add(
        await marketProxy.instruction.cancelOrderByClientId(
          program.provider.wallet.publicKey,
          openOrders,
          new BN(999)
        )
      );
      await provider.send(tx);

      // Then.
      const afterOoAccount = await OpenOrders.load(
        provider.connection,
        openOrders,
        DEX_PID
      );
      assert.ok(beforeOoAccount.quoteTokenFree.eq(new BN(0)));
      assert.ok(beforeOoAccount.quoteTokenTotal.eq(usdcPosted));
      assert.ok(afterOoAccount.quoteTokenFree.eq(usdcPosted));
      assert.ok(afterOoAccount.quoteTokenTotal.eq(usdcPosted));
    });

    // Need to crank the cancel so that we can close later.
    it("Cranks the cancel transaction", async () => {
      // TODO: can do this in a single transaction if we covert the pubkey bytes
      //       into a [u64; 4] array and sort. I'm lazy though.
      let eq = await marketProxy.market.loadEventQueue(provider.connection);
      while (eq.length > 0) {
        const tx = new Transaction();
        tx.add(
          marketProxy.market.makeConsumeEventsInstruction([eq[0].openOrders], 1)
        );
        await provider.send(tx);
        eq = await marketProxy.market.loadEventQueue(provider.connection);
      }
    });

    it("Settles funds on the orderbook", async () => {
      // Given.
      const beforeTokenAccount = await usdcClient.getAccountInfo(usdcAccount);

      // When.
      const tx = new Transaction();
      tx.add(
        await marketProxy.instruction.settleFunds(
          openOrders,
          provider.wallet.publicKey,
          tokenAccount,
          usdcAccount,
          referral
        )
      );
      await provider.send(tx);

      // Then.
      const afterTokenAccount = await usdcClient.getAccountInfo(usdcAccount);
      assert.ok(
        afterTokenAccount.amount.sub(beforeTokenAccount.amount).toNumber() ===
        usdcPosted.toNumber()
      );
    });

    it("Closes an open orders account", async () => {
      // Given.
      const beforeAccount = await program.provider.connection.getAccountInfo(
        program.provider.wallet.publicKey
      );

      // When.
      const tx = new Transaction();
      tx.add(
        marketProxy.instruction.closeOpenOrders(
          openOrders,
          provider.wallet.publicKey,
          provider.wallet.publicKey
        )
      );
      await provider.send(tx);

      // Then.
      const afterAccount = await program.provider.connection.getAccountInfo(
        program.provider.wallet.publicKey
      );
      const closedAccount = await program.provider.connection.getAccountInfo(
        openOrders
      );
      assert.ok(23352768 === afterAccount.lamports - beforeAccount.lamports);
      assert.ok(closedAccount === null);
    });
  });
});

// Dummy identity middleware used for testing.
class Identity {
  initOpenOrders(ix) {
    this.proxy(ix);
  }
  newOrderV3(ix) {
    this.proxy(ix);
  }
  cancelOrderV2(ix) {
    this.proxy(ix);
  }
  cancelOrderByClientIdV2(ix) {
    this.proxy(ix);
  }
  settleFunds(ix) {
    this.proxy(ix);
  }
  closeOpenOrders(ix) {
    this.proxy(ix);
  }
  proxy(ix) {
    ix.keys = [
      { pubkey: SYSVAR_RENT_PUBKEY, isWritable: false, isSigner: false },
      ...ix.keys,
    ];
  }
}
