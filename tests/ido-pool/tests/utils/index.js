const spl = require("@solana/spl-token");
const anchor = require("@anchor-lang/core");
const serumCmn = require("@project-serum/common");
const TokenInstructions = require("@project-serum/serum").TokenInstructions;

// TODO: remove this constant once @project-serum/serum uses the same version
//       of @solana/web3.js as anchor (or switch packages).
const TOKEN_PROGRAM_ID = new anchor.web3.PublicKey(
  TokenInstructions.TOKEN_PROGRAM_ID.toString()
);

// Our own sleep function.
function sleep(ms) {
  console.log("Sleeping for", ms / 1000, "seconds");
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// Read the cluster's current `unix_timestamp` — the same value the on-chain
// `Clock::get()` observes.
async function getClusterTime(connection) {
  for (let attempt = 0; attempt < 20; attempt++) {
    const slot = await connection.getSlot("confirmed");
    const time = await connection.getBlockTime(slot);
    if (time !== null) return time;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error("getBlockTime returned null for 20 consecutive slots");
}

// Jump the cluster clock to *strictly past* `targetUnixSecs` via surfpool's
// `surfnet_timeTravel` cheatcode. The on-chain phase checks all use
// `clock.unix_timestamp <= boundary` — a block whose `unix_timestamp` equals
// the boundary still trips the check, so we target `boundary + 1`.
//
// Driving cluster time via an RPC (instead of polling wall-clock pacing)
// decouples phase progression from tx confirmation latency: a stalled tx
// can no longer burn the phase budget and cascade-fail later tests.
//
// `absoluteTimestamp` is in milliseconds (surfpool divides by 1000 when
// writing the Clock sysvar's `unix_timestamp`).
async function advanceClusterTime(connection, targetUnixSecs) {
  const res = await connection._rpcRequest("surfnet_timeTravel", [
    { absoluteTimestamp: (targetUnixSecs + 1) * 1000 },
  ]);
  if (res.error) {
    throw new Error(`surfnet_timeTravel failed: ${JSON.stringify(res.error)}`);
  }
}

async function getTokenAccount(provider, addr) {
  return await serumCmn.getTokenAccount(provider, addr);
}

async function createMint(provider, authority) {
  if (authority === undefined) {
    authority = provider.wallet.publicKey;
  }
  const mint = await spl.Token.createMint(
    provider.connection,
    provider.wallet.payer,
    authority,
    null,
    6,
    TOKEN_PROGRAM_ID
  );
  return mint;
}

async function createTokenAccount(provider, mint, owner) {
  const token = new spl.Token(
    provider.connection,
    mint,
    TOKEN_PROGRAM_ID,
    provider.wallet.payer
  );
  let vault = await token.createAccount(owner);
  return vault;
}

module.exports = {
  TOKEN_PROGRAM_ID,
  sleep,
  getClusterTime,
  advanceClusterTime,
  getTokenAccount,
  createTokenAccount,
  createMint,
};
