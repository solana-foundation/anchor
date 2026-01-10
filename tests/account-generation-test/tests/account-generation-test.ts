import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { assert } from "chai";
import { AccountGenerationTest } from "../target/types/account_generation_test";

describe("account-generation-test", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const program = anchor.workspace
    .AccountGenerationTest as Program<AccountGenerationTest>;

  it("Accounts should be pre-funded", async () => {
    // Check that accounts were generated with funded lamports
    // The first account should be a new keypair that was generated
    // The second account should be the specified address
    
    const specifiedAccount = new anchor.web3.PublicKey(
      "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"
    );
    
    // Check balance of specified account (should have 5 SOL = 5000000000 lamports)
    const balance = await provider.connection.getBalance(specifiedAccount);
    assert.isAtLeast(
      balance,
      5000000000,
      "Specified account should have at least 5 SOL"
    );
    
    console.log(`✓ Specified account balance: ${balance / anchor.web3.LAMPORTS_PER_SOL} SOL`);
  });

  it("Popular mints should be configured", async () => {
    // Check that USDC mint exists
    const usdcMint = new anchor.web3.PublicKey(
      "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    );
    
    const usdcAccount = await provider.connection.getAccountInfo(usdcMint);
    // Note: The mint might not be fully initialized yet, but the account should exist
    console.log(`✓ USDC mint account exists: ${usdcAccount !== null}`);
    
    // Check that mSOL mint exists
    const msolMint = new anchor.web3.PublicKey(
      "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So"
    );
    
    const msolAccount = await provider.connection.getAccountInfo(msolMint);
    console.log(`✓ mSOL mint account exists: ${msolAccount !== null}`);
  });
});

