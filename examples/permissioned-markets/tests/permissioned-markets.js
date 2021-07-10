const assert = require("assert");
const { Token, TOKEN_PROGRAM_ID } = require("@solana/spl-token");
const anchor = require("@project-serum/anchor");
const serum = require("@project-serum/serum");
const { BN } = anchor;
const { Transaction, TransactionInstruction } = anchor.web3;
const { DexInstructions, OpenOrders, Market } = serum;
const { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } = anchor.web3;
const { initMarket, sleep } = require("./utils");

const DEX_PID = new PublicKey("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");
const REFERRAL = new PublicKey("2k1bb16Hu7ocviT2KC3wcCgETtnC8tEUuvFBH4C5xStG");

describe("permissioned-markets", () => {
  // Anchor client setup.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.PermissionedMarkets;

  // Token clients.
  let usdcClient;

  // Global DEX accounts and clients shared accross all tests.
  let marketClient, tokenAccount, usdcAccount;
  let openOrders, openOrdersBump, openOrdersInitAuthority, openOrdersBumpinit;
  let usdcPosted;
  let marketMakerOpenOrders;

  it("BOILERPLATE: Initializes an orderbook", async () => {
    const {
      marketMakerOpenOrders: mmOo,
      marketA,
      godA,
      godUsdc,
      usdc,
    } = await initMarket({ provider });
    marketClient = marketA;
    marketClient._programId = program.programId;
    usdcAccount = godUsdc;
    tokenAccount = godA;
    marketMakerOpenOrders = mmOo;

    usdcClient = new Token(
      provider.connection,
      usdc,
      TOKEN_PROGRAM_ID,
      provider.wallet.payer
    );
  });

  it("BOILERPLATE: Calculates open orders addresses", async () => {
    const [_openOrders, bump] = await PublicKey.findProgramAddress(
      [
        anchor.utils.bytes.utf8.encode("open-orders"),
        marketClient.address.toBuffer(),
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
        marketClient.address.toBuffer(),
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
    await program.rpc.initAccount(openOrdersBump, openOrdersBumpInit, {
      accounts: {
        openOrdersInitAuthority,
        openOrders,
        authority: program.provider.wallet.publicKey,
        market: marketClient.address,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
        dexProgram: DEX_PID,
      },
    });

    const account = await provider.connection.getAccountInfo(openOrders);
    assert.ok(account.owner.toString() === DEX_PID.toString());
  });

  it("Posts a bid on the orderbook", async () => {
    const size = 1;
    const price = 1;

    // The amount of USDC transferred into the dex for the trade.
    usdcPosted = new BN(marketClient._decoded.quoteLotSize.toNumber()).mul(
      marketClient
        .baseSizeNumberToLots(size)
        .mul(marketClient.priceNumberToLots(price))
    );

    // Note: Prepend delegate approve to the tx since the owner of the token
    //       account must match the owner of the open orders account. We
    //       can probably hide this in the serum client.
    const tx = new Transaction();
    tx.add(
      Token.createApproveInstruction(
        TOKEN_PROGRAM_ID,
        usdcAccount,
        openOrders,
        program.provider.wallet.publicKey,
        [],
        usdcPosted.toNumber()
      )
    );
    tx.add(
      serumProxy(
        marketClient.makePlaceOrderInstruction(program.provider.connection, {
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
      )
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
      serumProxy(
        (
          await marketClient.makeCancelOrderByClientIdTransaction(
            program.provider.connection,
            program.provider.wallet.publicKey,
            openOrders,
            new BN(999)
          )
        ).instructions[0]
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
    let eq = await marketClient.loadEventQueue(provider.connection);
    while (eq.length > 0) {
      const tx = new Transaction();
      tx.add(
        DexInstructions.consumeEvents({
          market: marketClient._decoded.ownAddress,
          eventQueue: marketClient._decoded.eventQueue,
          coinFee: marketClient._decoded.eventQueue,
          pcFee: marketClient._decoded.eventQueue,
          openOrdersAccounts: [eq[0].openOrders],
          limit: 1,
          programId: DEX_PID,
        })
      );
      await provider.send(tx);
      eq = await marketClient.loadEventQueue(provider.connection);
    }
  });

  it("Settles funds on the orderbook", async () => {
    // Given.
    const beforeTokenAccount = await usdcClient.getAccountInfo(usdcAccount);

    // When.
    const tx = new Transaction();
    tx.add(
      serumProxy(
        DexInstructions.settleFunds({
          market: marketClient._decoded.ownAddress,
          openOrders,
          owner: provider.wallet.publicKey,
          baseVault: marketClient._decoded.baseVault,
          quoteVault: marketClient._decoded.quoteVault,
          baseWallet: tokenAccount,
          quoteWallet: usdcAccount,
          vaultSigner: await PublicKey.createProgramAddress(
            [
              marketClient.address.toBuffer(),
              marketClient._decoded.vaultSignerNonce.toArrayLike(
                Buffer,
                "le",
                8
              ),
            ],
            DEX_PID
          ),
          programId: program.programId,
          referrerQuoteWallet: usdcAccount,
        })
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
      serumProxy(
        DexInstructions.closeOpenOrders({
          market: marketClient._decoded.ownAddress,
          openOrders,
          owner: program.provider.wallet.publicKey,
          solWallet: program.provider.wallet.publicKey,
          programId: program.programId,
        })
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

// Adds the serum dex account to the instruction so that proxies can
// relay (CPI requires the executable account).
//
// TODO: we should add flag in the dex client that says if a proxy is being
//       used, and if so, do this automatically.
function serumProxy(ix) {
  ix.keys = [
    { pubkey: DEX_PID, isWritable: false, isSigner: false },
    ...ix.keys,
  ];
  return ix;
}
