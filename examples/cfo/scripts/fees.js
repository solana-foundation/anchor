#!/usr/bin/env node

const process = require("process");
const fs = require("fs");
const anchor = require("@project-serum/anchor");
const { Market, OpenOrders } = require("@project-serum/serum");
const Account = anchor.web3.Account;
const Program = anchor.Program;
const provider = anchor.Provider.local();
const secret = JSON.parse(fs.readFileSync("./scripts/market-maker.json"));
const MARKET_MAKER = new Account(secret);
const PublicKey = anchor.web3.PublicKey;

const DEX_PID = new PublicKey("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");

async function main() {
  const market = new PublicKey(process.argv[2]);
  while (true) {
    let marketClient = await Market.load(
      provider.connection,
      market,
      { commitment: "recent" },
      DEX_PID
    );
    console.log("Fees: ", marketClient._decoded.quoteFeesAccrued.toString());
    await sleep(3000);
  }
}

main();

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
