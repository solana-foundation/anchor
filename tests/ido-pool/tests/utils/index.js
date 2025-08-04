const {
  TOKEN_PROGRAM_ID,
  getAccount,
  createMint,
  createAccount,
} = require("@solana/spl-token");
const { Keypair } = require("@solana/web3.js");

// Our own sleep function.
function sleep(ms) {
  console.log("Sleeping for", ms / 1000, "seconds");
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function getTokenAccount(provider, addr) {
  return await getAccount(provider.connection, addr);
}

async function createTokenMint(provider, authority) {
  if (authority === undefined) {
    authority = provider.wallet.publicKey;
  }
  const mint = await createMint(
    provider.connection,
    provider.wallet.payer,
    authority,
    null,
    6
  );
  return mint;
}

async function createTokenAccount(provider, mint, owner) {
  let vault = await createAccount(
    provider.connection,
    provider.wallet.payer,
    mint,
    owner,
    Keypair.generate()
  );
  return vault;
}

module.exports = {
  sleep,
  getTokenAccount,
  createTokenAccount,
  createTokenMint,
};
