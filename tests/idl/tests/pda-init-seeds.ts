import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";

import type { PdaInitSeeds } from "../target/types/pda_init_seeds";

describe("PDA Init Seeds IDL", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.pdaInitSeeds as anchor.Program<PdaInitSeeds>;

  it("Includes PDA field for pool account with init + seeds", () => {
    const initializePoolIx = program.idl.instructions.find(
      (ix) => ix.name === "initialize_pool"
    );
    if (!initializePoolIx) {
      throw new Error("Instruction `initializePool` not found");
    }

    // Find the pool account in the instruction
    const poolAccount = initializePoolIx.accounts.find(
      (acc) => acc.name === "pool"
    );
    if (!poolAccount || "accounts" in poolAccount) {
      throw new Error("Account `pool` not found or is composite");
    }

    // Verify the PDA field is present
    assert.isDefined(
      poolAccount.pda,
      "PDA field should be present for pool account with init + seeds"
    );

    // Verify the PDA seeds structure
    assert.isDefined(poolAccount.pda?.seeds, "PDA seeds should be defined");
    assert.isArray(poolAccount.pda?.seeds, "PDA seeds should be an array");

    const seeds = poolAccount.pda!.seeds;
    assert.strictEqual(seeds.length, 3, "Pool should have 3 seeds");

    // First seed: constant "pool"
    assert.strictEqual(seeds[0].kind, "const", "First seed should be const");
    if (seeds[0].kind === "const") {
      const poolBytes = Buffer.from("pool");
      assert.deepEqual(
        seeds[0].value,
        Array.from(poolBytes),
        "First seed should be 'pool' bytes"
      );
    }

    // Second seed: token_a_mint account
    assert.strictEqual(
      seeds[1].kind,
      "account",
      "Second seed should be account"
    );
    if (seeds[1].kind === "account") {
      assert.strictEqual(
        seeds[1].path,
        "token_a_mint",
        "Second seed path should be 'token_a_mint'"
      );
    }

    // Third seed: token_b_mint account
    assert.strictEqual(
      seeds[2].kind,
      "account",
      "Third seed should be account"
    );
    if (seeds[2].kind === "account") {
      assert.strictEqual(
        seeds[2].path,
        "token_b_mint",
        "Third seed path should be 'token_b_mint'"
      );
    }
  });

  it("Includes PDA field for position account with init + seeds", () => {
    const initializePositionIx = program.idl.instructions.find(
      (ix) => ix.name === "initialize_position"
    );
    if (!initializePositionIx) {
      throw new Error("Instruction `initializePosition` not found");
    }

    // Find the position account in the instruction
    const positionAccount = initializePositionIx.accounts.find(
      (acc) => acc.name === "position"
    );
    if (!positionAccount || "accounts" in positionAccount) {
      throw new Error("Account `position` not found or is composite");
    }

    // Verify the PDA field is present
    assert.isDefined(
      positionAccount.pda,
      "PDA field should be present for position account with init + seeds"
    );

    // Verify the PDA seeds structure
    assert.isDefined(
      positionAccount.pda?.seeds,
      "PDA seeds should be defined"
    );
    assert.isArray(
      positionAccount.pda?.seeds,
      "PDA seeds should be an array"
    );

    const seeds = positionAccount.pda!.seeds;
    assert.strictEqual(seeds.length, 2, "Position should have 2 seeds");

    // First seed: constant "position"
    assert.strictEqual(seeds[0].kind, "const", "First seed should be const");
    if (seeds[0].kind === "const") {
      const positionBytes = Buffer.from("position");
      assert.deepEqual(
        seeds[0].value,
        Array.from(positionBytes),
        "First seed should be 'position' bytes"
      );
    }

    // Second seed: position_nft_mint account
    assert.strictEqual(
      seeds[1].kind,
      "account",
      "Second seed should be account"
    );
    if (seeds[1].kind === "account") {
      assert.strictEqual(
        seeds[1].path,
        "position_nft_mint",
        "Second seed path should be 'position_nft_mint'"
      );
    }
  });

  it("Includes PDA field for customizable pool with init + seeds using const prefix", () => {
    const initializeCustomizablePoolIx = program.idl.instructions.find(
      (ix) => ix.name === "initialize_customizable_pool"
    );
    if (!initializeCustomizablePoolIx) {
      throw new Error("Instruction `initializeCustomizablePool` not found");
    }

    // Find the pool account in the instruction
    const poolAccount = initializeCustomizablePoolIx.accounts.find(
      (acc) => acc.name === "pool"
    );
    if (!poolAccount || "accounts" in poolAccount) {
      throw new Error("Account `pool` not found or is composite");
    }

    // Verify the PDA field is present
    assert.isDefined(
      poolAccount.pda,
      "PDA field should be present for customizable pool with init + seeds"
    );

    // Verify the PDA seeds structure
    assert.isDefined(poolAccount.pda?.seeds, "PDA seeds should be defined");
    assert.isArray(poolAccount.pda?.seeds, "PDA seeds should be an array");

    const seeds = poolAccount.pda!.seeds;
    assert.strictEqual(seeds.length, 3, "Customizable pool should have 3 seeds");

    // First seed: constant "cpool" (from POOL_PREFIX const)
    assert.strictEqual(seeds[0].kind, "const", "First seed should be const");
    if (seeds[0].kind === "const") {
      const cpoolBytes = Buffer.from("cpool");
      assert.deepEqual(
        seeds[0].value,
        Array.from(cpoolBytes),
        "First seed should be 'cpool' bytes"
      );
    }

    // Second seed: token_a_mint account
    assert.strictEqual(
      seeds[1].kind,
      "account",
      "Second seed should be account"
    );
    if (seeds[1].kind === "account") {
      assert.strictEqual(
        seeds[1].path,
        "token_a_mint",
        "Second seed path should be 'token_a_mint'"
      );
    }

    // Third seed: token_b_mint account
    assert.strictEqual(
      seeds[2].kind,
      "account",
      "Third seed should be account"
    );
    if (seeds[2].kind === "account") {
      assert.strictEqual(
        seeds[2].path,
        "token_b_mint",
        "Third seed path should be 'token_b_mint'"
      );
    }
  });
});

