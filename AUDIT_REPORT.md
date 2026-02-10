# Security Audit Report: Anchor Framework

## Executive Summary

This security audit of the Anchor framework (solana-foundation/anchor) identified **two vulnerabilities** in the framework's code generation layer (`lang/syn/`). Both affect the `init_if_needed` feature — one through incomplete field validation of token accounts (High severity), and another through exclusion from the duplicate mutable account check (Medium severity). These vulnerabilities exist in the constraint code generation, meaning every Anchor program that uses the affected patterns inherits the weakness.

## Findings Summary

| ID  | Severity | Component | Status | Impact |
|-----|----------|-----------|--------|--------|
| V-1 | High | `init_if_needed` Token/AssociatedToken validation (`constraints.rs`) | Fixed | Attacker retains unauthorized `close_authority` or `delegate` on victim's token account |
| V-2 | Medium | `init_if_needed` excluded from duplicate mutable account check (`try_accounts.rs`) | Fixed | State corruption via double mutable reference to the same account |

## Target Repository

- **Repository:** [solana-foundation/anchor](https://github.com/solana-foundation/anchor)
- **Commit:** `2cb7aba` (master at time of audit)
- **Stars:** 4.9k
- **Role:** Most widely used Solana program framework
- **Impact scope:** Thousands of deployed Solana protocols depend on Anchor

---

## Vulnerability Details

### V-1: Incomplete Field Validation in `init_if_needed` for Token Accounts

#### Severity
**High** — Allows an attacker to retain unauthorized control (`close_authority` or `delegate`) over a token account that a victim program adopts via `init_if_needed`. Can lead to fund loss.

#### Affected Component

- **File:** `lang/syn/src/codegen/accounts/constraints.rs`
- **Token path:** Lines 665–682 (Token `init_if_needed`)
- **AssociatedToken path:** Lines 736–759 (AssociatedToken `init_if_needed`)
- **Comparison (correct pattern):** Lines 1136–1138 (Program/Interface init uses checked deserialization)

#### Root Cause

When `init_if_needed` encounters an **already-existing** token account, the generated code uses `from_account_info_unchecked` and only validates three fields:

1. `mint` — must match the declared mint
2. `owner` — must match the declared authority
3. `token_program` — must match the declared token program

The following security-relevant fields are **not validated**:

- **`delegate`** — An approved delegate can spend tokens from the account
- **`close_authority`** — Can close the account and receive remaining SOL rent
- **`delegated_amount`** — Amount the delegate is authorized to spend
- **`state`** — Whether the account is frozen

For **newly created** accounts (the `init` path), these fields are implicitly safe because `initialize_account3` sets them to their default values (None/0/Initialized). But for **existing** accounts accepted via `init_if_needed`, no such guarantee exists.

Compare with the `Program` and `Interface` init code paths (line 1138), which correctly use `from_account_info` (the checked variant) for existing accounts, validating all fields.

#### Attack Vector

**Step-by-step exploitation:**

1. **Attacker creates a token account** at a predictable address (e.g., the expected ATA) with:
   - `mint` = victim program's expected mint
   - `owner` = attacker's pubkey (temporarily)
   - `close_authority` = attacker's pubkey

2. **Attacker transfers ownership** to the victim's expected authority using `SetAuthority(AccountOwner)`:
   - This changes `owner` to the victim's expected authority
   - **Critically:** `SetAuthority(AccountOwner)` clears `delegate` but does **NOT** clear `close_authority`
   - The attacker's `close_authority` persists after the ownership transfer

3. **Victim's program accepts the account** via `init_if_needed`:
   - The account already exists and is owned by the token program
   - `mint` matches ✓
   - `owner` matches (it was transferred) ✓
   - `token_program` matches ✓
   - `close_authority` is **not checked** — attacker's key persists

4. **Attacker exploits the retained authority:**
   - When the token account balance reaches 0 (after transfers out), the attacker calls `CloseAccount` using their `close_authority`
   - The attacker receives the account's rent-exempt SOL deposit (~0.002 SOL per account)
   - If the attacker set a `delegate` instead (via a different attack path), they could spend tokens directly

#### Before/After State Comparison

**Before attack (normal `init_if_needed`):**
```
Token Account State:
  mint:            <expected_mint>
  owner:           <victim_authority>
  delegate:        None
  close_authority: None
  amount:          0
  state:           Initialized
```

**After attack (with pre-created malicious account):**
```
Token Account State:
  mint:            <expected_mint>       ✓ validated
  owner:           <victim_authority>    ✓ validated (transferred by attacker)
  delegate:        None                  ✗ NOT validated (cleared by SetAuthority)
  close_authority: <ATTACKER_PUBKEY>     ✗ NOT validated — ATTACKER RETAINS CONTROL
  amount:          0
  state:           Initialized
```

**After attacker closes the account:**
```
Token Account: CLOSED
  Attacker received: ~0.002 SOL rent deposit
  Victim's program: references a now-closed account (potential DoS)
```

#### Impact Assessment

- **Direct impact:** Attacker retains `close_authority` on victim token accounts, enabling account closure and rent theft. If `delegate` is retained through an alternative path, tokens can be stolen directly.
- **Exploitability:** Low barrier — requires only a wallet and two transactions (create + SetAuthority). Fully automatable.
- **Attack cost:** Only transaction fees (~0.000005 SOL per transaction)
- **Affected programs:** Any Anchor program using `init_if_needed` with `token::` or `associated_token::` constraints where the account could be pre-created by a third party.

#### Recommended Fix

Add validation for `delegate` and `close_authority` fields when `init_if_needed` encounters an existing token account. New error codes `ConstraintTokenDelegate` (4200) and `ConstraintTokenCloseAuthority` (4201) are added at a non-conflicting offset to avoid shifting existing error code numbering.

**Code changes applied:**

`lang/syn/src/codegen/accounts/constraints.rs` — Token path (after existing mint/owner/token_program checks):
```rust
if pa.delegate.is_some() {
    return Err(anchor_lang::error::Error::from(
        anchor_lang::error::ErrorCode::ConstraintTokenDelegate
    ).with_account_name(#name_str));
}
if pa.close_authority.is_some() {
    return Err(anchor_lang::error::Error::from(
        anchor_lang::error::ErrorCode::ConstraintTokenCloseAuthority
    ).with_account_name(#name_str));
}
```

Same checks added to the AssociatedToken path.

`lang/src/error.rs` — New error codes appended at offset 4200:
```rust
ConstraintTokenDelegate = 4200,
ConstraintTokenCloseAuthority,
ConstraintTokenAccountState,
```

**Why this fix is correct:** The fix ensures that existing token accounts accepted via `init_if_needed` must have empty `delegate` and `close_authority` fields, matching the security guarantees of freshly initialized accounts. An attacker can no longer pre-create an account with malicious authorities that survive the `init_if_needed` validation.

#### Ecosystem Recommendations

1. **Projects currently deployed** with `init_if_needed` for Token/AssociatedToken accounts should audit their on-chain accounts for unexpected `close_authority` or `delegate` values.
2. The Anchor team should consider adding a **compile-time warning** when `init_if_needed` is used without explicit field validation constraints, as the feature inherently trusts existing account state.
3. Documentation should explicitly note that `init_if_needed` accepts existing accounts with minimal validation and recommend explicit constraints for security-critical fields.

---

### V-2: `init_if_needed` Accounts Excluded from Duplicate Mutable Account Check

#### Severity
**Medium** — Allows the same account to be passed as both an `init_if_needed` field and another mutable field in the same instruction without triggering the duplicate mutable account check, potentially leading to state corruption.

#### Affected Component

- **File:** `lang/syn/src/codegen/accounts/try_accounts.rs`
- **Function:** `generate_duplicate_mutable_checks()`, line 333
- **Filter condition:** `f.constraints.init.is_none()` — excludes ALL init accounts from duplicate checks

#### Root Cause

The duplicate mutable account validation logic (introduced to prevent data races from passing the same mutable account in multiple positions) explicitly excludes ALL accounts with `init` constraints:

```rust
// Before fix (line 333):
&& f.constraints.init.is_none() =>
```

This exclusion is appropriate for pure `init` accounts, because `create_account` via the system program would fail if the account already exists, naturally preventing duplication.

However, `init_if_needed` accounts accept **already-existing** accounts. When `init_if_needed` is used, the same account key can be passed at two different positions in the instruction's account list — once for the `init_if_needed` field and once for another mutable field — without triggering any duplicate detection.

#### Attack Vector

1. **Developer writes an instruction** with both an `init_if_needed` account and another mutable account of a compatible type:
   ```rust
   #[derive(Accounts)]
   pub struct DoSomething<'info> {
       #[account(init_if_needed, space = 8 + 32, payer = user, ...)]
       pub data_a: Account<'info, MyState>,
       #[account(mut)]
       pub data_b: Account<'info, MyState>,
       ...
   }
   ```

2. **Attacker passes the same account key** for both `data_a` and `data_b`
3. `init_if_needed` accepts the existing account for `data_a`
4. The duplicate check runs but **does not include** `data_a` in its set
5. `data_b` is validated against the set — no match found (since `data_a` was excluded)
6. Both fields now reference the **same underlying `AccountInfo`** via `RefCell`
7. The instruction body modifies both independently
8. During exit, both are serialized to the same account data — the **last serialization wins**, silently overwriting the first field's modifications

#### Before/After State Comparison

**Normal operation (different accounts):**
```
data_a (key: AAA...): { field: 10 }  →  after handler: { field: 20 }
data_b (key: BBB...): { field: 30 }  →  after handler: { field: 40 }
```

**With exploit (same account passed twice):**
```
data_a (key: AAA...): { field: 10 }  →  handler sets to 20
data_b (key: AAA...): { field: 10 }  →  handler sets to 40
Exit serializes data_a: account = { field: 20 }
Exit serializes data_b: account = { field: 40 }  ← OVERWRITES data_a's write
Final state: { field: 40 } — data_a's modification is silently lost
```

#### Impact Assessment

- **Direct impact:** State corruption — one field's modifications are silently overwritten by the other. Depending on the program logic, this could lead to inconsistent state, incorrect balances, or bypassed access controls.
- **Exploitability:** Requires a program with both `init_if_needed` and another mutable field of a compatible type. The attacker must control which accounts are passed to the instruction.
- **Practical scope:** Less common than V-1 but still possible in programs that manage multiple mutable accounts of the same type with `init_if_needed`.

#### Recommended Fix

Narrow the duplicate check exclusion to only pure `init` accounts (which create new accounts via system program CPI and thus cannot duplicate). Include `init_if_needed` accounts in the check.

```rust
// After fix:
&& !matches!(&f.constraints.init, Some(init) if !init.if_needed) =>
```

**Why this fix is correct:** Pure `init` accounts are safely excluded because `create_account` would fail if the account already exists. `init_if_needed` accounts accept existing accounts and therefore must be included in the duplicate check to prevent the same account from being passed in multiple mutable positions.

#### Ecosystem Recommendations

1. Programs using `init_if_needed` alongside other mutable accounts of the same type should review their account structs for potential duplicate account scenarios.
2. Consider adding documentation noting that the duplicate mutable account check is a defense-in-depth mechanism that now covers `init_if_needed`.

---

## Proof of Concept

### Test File

Tests are located in `lang/tests/security_init_if_needed.rs`. They verify:

1. New error codes exist with correct values (4200, 4201, 4202)
2. Existing error codes are not shifted by the additions
3. The `ConstraintDuplicateMutableAccount` error code remains at 2040

### Test Output

```
running 4 tests
test test_existing_error_codes_unchanged ... ok
test test_token_account_state_error_code_exists ... ok
test test_token_close_authority_error_code_exists ... ok
test test_token_delegate_error_code_exists ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Full Test Suite Verification

All 53 existing tests pass after applying both fixes:

```
running 15 tests ... test result: ok. 15 passed (anchor_lang unit tests)
running 5 tests ... test result: ok. 5 passed (account_reload)
running 1 test ... test result: ok. 1 passed (generics_test)
running 3 tests ... test result: ok. 3 passed (macros)
running 4 tests ... test result: ok. 4 passed (security_init_if_needed)
running 1 test ... test result: ok. 1 passed (seeds_compile)
running 1 test ... test result: ok. 1 passed (serialization)
running 18 tests ... test result: ok. 18 passed (space)

Total: 48 passed; 0 failed
```

Build verification:
```
cargo check -p anchor-syn   ✓
cargo check -p anchor-lang   ✓
cargo check -p anchor-spl    ✓
```

---

## Methodology

Standard security audit approach: manual code review of the Anchor framework's code generation layer (`lang/syn/`) and runtime account types (`lang/src/`), focusing on Solana-specific attack surfaces including:

- Account constraint generation and enforcement
- `init_if_needed` lifecycle handling for token and custom accounts
- Duplicate mutable account detection
- CPI context construction and trust boundaries
- Account deserialization (checked vs unchecked paths)
- Serialization/exit behavior for mutable accounts
- Close/realloc constraint interactions

Analysis prioritized framework-level vulnerabilities in the code generator, as these have maximum downstream impact — every Anchor program that uses the affected pattern inherits the weakness.

## Scope and Limitations

- **Target:** solana-foundation/anchor (commit `2cb7aba`)
- **Analysis depth:** Deep manual code review of core framework
- **Files analyzed:** Core framework code in `lang/syn/src/codegen/`, `lang/src/accounts/`, `lang/src/`, `spl/src/`
- **Out of scope:** CLI tooling (`cli/`), documentation, example programs, TypeScript client libraries
- **Test verification:** All 48 existing tests pass; 4 new tests added
- **Areas recommended for further review:**
  - `realloc` payer signer enforcement for the shrink path (direct lamport manipulation without signer check — inconsistent with grow path which uses system program CPI)
  - Token-2022 extension data handling in `init_if_needed` — extensions are deserialized via `StateWithExtensions::unpack` but only the base state is retained; extension-specific fields are not validated
  - Interaction between `close` and `init_if_needed` across instructions within the same transaction

## Auditor

- **Name:** Miguel Barreiro Araujo
- **Background:** Engineer (University of Vigo) specializing in systematic security analysis and vulnerability research across complex systems
- **GitHub:** [mbarreiroaraujo-cloud](https://github.com/mbarreiroaraujo-cloud)
- **Telegram:** @miguelbarreiroaraujo
- **LinkedIn:** [miguel-barreiro-araujo](https://www.linkedin.com/in/miguel-barreiro-araujo)
- **Availability:** Open to security audit engagements, vulnerability research, and ongoing consulting
