#!/usr/bin/env node

// Script to list a market, logging the address to stdout.

const utils = require("../tests/utils");
const fs = require("fs");
const anchor = require("@project-serum/anchor");
const provider = anchor.Provider.local();

async function main() {
  ORDERBOOK_ENV = await utils.initMarket({
    provider,
  });
  const out = {
    market: ORDERBOOK_ENV.marketA._decoded.ownAddress.toString(),
  };
  console.log(JSON.stringify(out));
}

main();
