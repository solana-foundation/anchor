## TransactionBuilder guide

This document is a practical walkthrough of `TransactionBuilder`, the core utility for building and broadcasting Bitcoin transactions when interacting with **Saturn / Arch** programs.

---

### 1. Motivation

`TransactionBuilder` wraps a raw Bitcoin `Transaction` together with the extra metadata that an Arch program needs:

-   A list of accounts whose data **changes** (`modified_accounts`).
-   The set of **inputs that still need to be signed** (`inputs_to_sign`).
-   Mempool ancestry information (`tx_statuses`) so fee‐rate checks can take ancestor transactions into account.

The builder hides the housekeeping that would otherwise have to be repeated in every instruction implementation (fee calculation, input bookkeeping, state-transition plumbing, …).

---

### 2. Type signature & generics

```rust,ignore
pub struct TransactionBuilder<'a,
    const MAX_MODIFIED_ACCOUNTS: usize,
    const MAX_INPUTS_TO_SIGN: usize>
```

-   `MAX_MODIFIED_ACCOUNTS` – upper bound for how many program accounts may be touched.
-   `MAX_INPUTS_TO_SIGN` – upper bound for inputs that will still need signatures.

These limits are enforced at **compile-time** by means of a `FixedList`, so choose values large enough for the instruction you are implementing.

```rust,ignore
// Most instructions only modify a handful of accounts and sign a couple
// of inputs, so small constants suffice.
const MAX_MODIFIED_ACCOUNTS: usize = 8;
const MAX_INPUTS_TO_SIGN: usize = 4;
let mut builder: TransactionBuilder<MAX_MODIFIED_ACCOUNTS, MAX_INPUTS_TO_SIGN> = TransactionBuilder::new();
```

---

### 3. Typical workflow

Below is the canonical flow for composing an instruction‐level Bitcoin TX.

```rust,ignore
use saturn_bitcoin_transactions::{TransactionBuilder, fee_rate::FeeRate, constants::DUST_LIMIT};
use bitcoin::{TxOut, Amount, ScriptBuf};

// 1. Instantiate the builder -------------------------------------------------
let mut builder: TransactionBuilder<8, 4> = TransactionBuilder::new();

// 2. Add program-level inputs -------------------------------------------------
// A) state-transition for an existing account
builder.add_state_transition(&account_info);

// B) spend a regular UTXO owned by `program_info_pubkey`
builder.add_tx_input(&utxo_info, &tx_status, &program_info_pubkey);

// 3. Add outputs -------------------------------------------------------------
let payment = TxOut {
    value: Amount::from_sat(DUST_LIMIT + 42),
    script_pubkey: user_script.clone(),
};

builder.transaction.output.push(payment);

// 4. Make sure there is enough input value & pay the fees --------------------
let fee_rate = FeeRate::try_from(15.0).expect("valid fee rate in sat/vB");

builder.adjust_transaction_to_pay_fees(&fee_rate, Some(change_script.clone()))?;

// 5. Validate fee rate -------------------------------------------------------
builder.is_fee_rate_valid(&fee_rate)?;

// 6. Freeze the builder -------
builder.finalize()?;
```

> **Hint:** The order is flexible; feel free to add inputs & outputs in any sequence. What matters is calling `adjust_transaction_to_pay_fees` once you know the final shape of the transaction.

---

### 4. Frequently used helper methods

| Method                                             | What it does                                                                                                                                                                                                                |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `create_state_account`                             | Creates a brand-new PDA/state account and appends the corresponding state-transition input automatically.                                                                                                                   |
| `find_btc_in_utxos`                                | Greedy selection of UTXOs until a target amount is reached (prefers high-value UTXOs and, when enabled, avoids consolidation candidates). Returns the **indices** of the chosen `UtxoInfo`s plus the total selected amount. |
| `estimate_final_tx_vsize`                          | Returns the vsize (virtual size in vB) the final signed transaction will have. Useful for fee estimation before the transaction is complete.                                                                                |
| `estimate_tx_vsize_with_additional_inputs_outputs` | Same as above but allows you to pass a preview of _yet-to-be-added_ inputs/outputs to check the impact on size & fees.                                                                                                      |
| `get_fee_paid`                                     | Computes the current fee by subtracting output value from accumulated input value.                                                                                                                                          |

---

### 5. Handling UTXO consolidation (feature `utxo-consolidation`)

When the **`utxo-consolidation`** feature is compiled in, `TransactionBuilder` exposes an additional helper:

```rust,ignore
add_consolidation_utxos(pool_pubkey, fee_rate, pool_utxos, draft_changes)
```

This will opportunistically attach small _consolidation_ inputs owned by the pool whenever the economics make sense. The extra size and input amount are tracked in `extra_tx_size_for_consolidation` and `total_btc_consolidation_input` respectively. You can charge the program for that fee via `get_fee_paid_by_program`.

---

### 6. Error handling

Almost every method returns a `Result<_, BitcoinTxError>` (or `ProgramError` for Arch specific calls). The most common failure modes are:

-   `BitcoinTxError::NotEnoughBtcInPool` – selection algorithms could not gather the requested value.
-   `BitcoinTxError::InvalidFeeRateTooLow` – fee rate check after adding ancestors failed.
-   `BitcoinTxError::InsufficientInputAmount` – outputs exceed current inputs.

Use `?` to propagate them upward from your instruction implementation.

---

### 7. Unit tests reference

`src/lib.rs` contains an extensive test-suite demonstrating individual pieces of behaviour. If in doubt, open the module and search for the scenario you are trying to replicate – chances are a dedicated test already exists.

---

### 8. Advanced tips

1. **Replacing a full transaction**: Have a pre-signed template? Use `replace_transaction` to swap it in and let the builder deduce mempool information & totals.
2. **Manual input insertion**: `insert_tx_input`/`insert_state_transition_input` allow precise placement (important when input order has on-chain meaning, e.g. for inscriptions).
3. **Rune support (feature `runes`)**: When compiled with the feature, the builder also tracks `total_rune_input` and takes rune amounts into account during UTXO selection.

### 9. End-to-end example – Zero → One swap

Sometimes reading snippets is not enough. If you prefer a full, runnable demonstration open:

```text
crates/saturn-bitcoin-transactions/examples/zero_to_one_swap.rs
```

The file walks through, step-by-step, how to:

1. Mock the Arch runtime accounts and fee-oracle inputs.
2. Instantiate `TransactionBuilder`.
3. Add the state-transition input for the liquidity-pool shard.
4. Insert the user's Rune input plus the BTC output that pays them back.
5. Pull BTC liquidity from the pool, pay the swap, and let the builder create/change outputs to meet a `FeeRate` target.
6. Finalise the builder so Arch can collect the required signatures.

The code is not runnable (given that it make calls to Arch syscalls), but it is a good example of how to use the library.

You will see the raw unsigned transaction hex together with the list of input indices + pubkeys that must sign. The example intentionally keeps the numbers small and hard-codes the UTXOs so you can tweak them and observe how size / fee calculation behaves.

---
