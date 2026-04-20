//! Test program covering the derive's account-constraint surface.
//!
//! One handler per constraint variant, keyed off a 1-byte discriminator.
//! Each `#[derive(Accounts)]` struct exercises one constraint in
//! isolation so the integration tests can trip it from a known state.
//!
//! Covered:
//!   - `address = expr` + `address = expr @ MyErr`
//!   - `has_one = field` + `has_one = field @ MyErr`
//!   - `owner = expr` + `owner = expr @ MyErr`
//!   - `constraint = expr` + `constraint = expr @ MyErr`
//!   - `executable`
//!   - `rent_exempt = enforce`  (compile-only struct; see note)
//!   - `close = receiver`       (happy + self-close rejection)
//!   - `seeds::program = other` (cross-program PDA derivation)
//!   - `init_if_needed`         (create + reuse)
//!   - `zeroed`                 (pre-zeroed disc + non-zero rejection)
//!   - `#[account(signer)]` on `UncheckedAccount`

use anchor_lang_v2::prelude::*;

declare_id!("Con9ukTn9BRPXWcjS2UBbuN3NnCwy1hcaDNZ9Hb8QMNp");

/// Dummy program id used as the derivation domain for the
/// `seeds::program = OTHER_PROGRAM` override test. The PDA only has to be
/// verifiable under this key — it is never actually invoked.
pub const OTHER_PROGRAM: Address = Address::from_str_const(
    "Gue5TpR6sstSyGhSvmVeH2TeKqBYYqmXpRCacB9jAk8u",
);

/// Expected address for the `address = PINNED_ADDRESS` check.
/// Pinned to a known off-curve pubkey — tests pass this exact address
/// on the happy path and a different one on the violation path.
pub const PINNED_ADDRESS: Address = Address::from_str_const(
    "Pin1111111111111111111111111111111111111111",
);

// -- Custom error enum -------------------------------------------------------

#[error_code]
pub enum MyErr {
    #[msg("address did not match expected pinned value")]
    BadAddress,
    #[msg("has_one authority mismatch")]
    BadAuthority,
    #[msg("account is not owned by the expected program")]
    BadOwner,
    #[msg("arbitrary constraint expression was false")]
    BadConstraint,
}

// -- Account types -----------------------------------------------------------

#[account]
pub struct Data {
    pub authority: Address,
    pub value: u64,
}

// -- Handlers ----------------------------------------------------------------

#[program]
pub mod constraints {
    use super::*;

    /// Create a `Data` PDA at `[b"data"]` with `authority = ctx.accounts.authority`.
    /// Used by has_one + close + constraint tests as a pre-existing account.
    #[discrim = 0]
    pub fn initialize(ctx: &mut Context<Initialize>) -> Result<()> {
        ctx.accounts.data.authority = *ctx.accounts.authority.account().address();
        ctx.accounts.data.value = 42;
        Ok(())
    }

    #[discrim = 1]
    pub fn check_address(_ctx: &mut Context<CheckAddress>) -> Result<()> {
        Ok(())
    }

    #[discrim = 2]
    pub fn check_address_custom_err(
        _ctx: &mut Context<CheckAddressCustomErr>,
    ) -> Result<()> {
        Ok(())
    }

    #[discrim = 3]
    pub fn check_has_one(_ctx: &mut Context<CheckHasOne>) -> Result<()> {
        Ok(())
    }

    #[discrim = 4]
    pub fn check_has_one_custom_err(
        _ctx: &mut Context<CheckHasOneCustomErr>,
    ) -> Result<()> {
        Ok(())
    }

    #[discrim = 5]
    pub fn check_owner(_ctx: &mut Context<CheckOwner>) -> Result<()> {
        Ok(())
    }

    #[discrim = 6]
    pub fn check_owner_custom_err(
        _ctx: &mut Context<CheckOwnerCustomErr>,
    ) -> Result<()> {
        Ok(())
    }

    #[discrim = 7]
    pub fn check_constraint(_ctx: &mut Context<CheckConstraint>) -> Result<()> {
        Ok(())
    }

    #[discrim = 8]
    pub fn check_constraint_custom_err(
        _ctx: &mut Context<CheckConstraintCustomErr>,
    ) -> Result<()> {
        Ok(())
    }

    #[discrim = 9]
    pub fn check_executable(_ctx: &mut Context<CheckExecutable>) -> Result<()> {
        Ok(())
    }

    /// Close `data`, sending its lamports to `receiver`.
    #[discrim = 10]
    pub fn do_close(_ctx: &mut Context<DoClose>) -> Result<()> {
        Ok(())
    }

    /// PDA derived against `OTHER_PROGRAM` rather than this program's id.
    #[discrim = 11]
    pub fn check_seeds_program(_ctx: &mut Context<CheckSeedsProgram>) -> Result<()> {
        Ok(())
    }

    /// First call creates the PDA; subsequent calls reuse it.
    #[discrim = 12]
    pub fn do_init_if_needed(ctx: &mut Context<DoInitIfNeeded>) -> Result<()> {
        ctx.accounts.data.value = ctx.accounts.data.value.wrapping_add(1);
        Ok(())
    }

    /// Expects a pre-allocated account whose first 8 bytes are zero.
    #[discrim = 13]
    pub fn check_zeroed(_ctx: &mut Context<CheckZeroed>) -> Result<()> {
        Ok(())
    }

    /// `signer` attribute on an `UncheckedAccount` — a distinct code path
    /// from the native `Signer` type check.
    #[discrim = 14]
    pub fn check_signer(_ctx: &mut Context<CheckSigner>) -> Result<()> {
        Ok(())
    }
}

// -- Accounts structs --------------------------------------------------------

/// Init a Data PDA keyed by `[b"data"]`.
#[derive(Accounts)]
pub struct Initialize {
    #[account(mut)]
    pub payer: Signer,
    #[account(init, payer = payer, seeds = [b"data"], bump)]
    pub data: Account<Data>,
    /// Becomes the `authority` field of `Data`.
    pub authority: UncheckedAccount,
    pub system_program: Program<System>,
}

// 1. address = PINNED_ADDRESS
#[derive(Accounts)]
pub struct CheckAddress {
    #[account(address = PINNED_ADDRESS)]
    pub pinned: UncheckedAccount,
}

// 2. address = PINNED_ADDRESS @ MyErr::BadAddress
#[derive(Accounts)]
pub struct CheckAddressCustomErr {
    #[account(address = PINNED_ADDRESS @ MyErr::BadAddress)]
    pub pinned: UncheckedAccount,
}

// 3. has_one = authority
//
// `has_one` is deprecated in v2 in favour of `#[account(address = ...)]`
// on the referencing field. The derive-generated `TryAccounts` impl lives
// as a sibling of the struct, so `#[expect(deprecated)]` must cover the
// whole submodule to catch the warning emitted at the `has_one` keyword.
#[expect(deprecated)]
mod has_one_structs {
    use super::*;

    #[derive(Accounts)]
    pub struct CheckHasOne {
        #[account(has_one = authority)]
        pub data: Account<Data>,
        pub authority: UncheckedAccount,
    }

    // 4. has_one = authority @ MyErr::BadAuthority
    #[derive(Accounts)]
    pub struct CheckHasOneCustomErr {
        #[account(has_one = authority @ MyErr::BadAuthority)]
        pub data: Account<Data>,
        pub authority: UncheckedAccount,
    }
}
// Re-export everything so the `__client_accounts_*` modules the `#[program]`
// macro references (via `super::__client_accounts_<name>`) resolve.
pub use has_one_structs::*;

// 5. owner = System
#[derive(Accounts)]
pub struct CheckOwner {
    #[account(owner = System::id())]
    pub target: UncheckedAccount,
}

// 6. owner = System @ MyErr::BadOwner
#[derive(Accounts)]
pub struct CheckOwnerCustomErr {
    #[account(owner = System::id() @ MyErr::BadOwner)]
    pub target: UncheckedAccount,
}

// 7. constraint = a.address() != b.address()
#[derive(Accounts)]
pub struct CheckConstraint {
    pub a: UncheckedAccount,
    #[account(constraint = a.address() != b.address())]
    pub b: UncheckedAccount,
}

// 8. constraint = ... @ MyErr::BadConstraint
#[derive(Accounts)]
pub struct CheckConstraintCustomErr {
    pub a: UncheckedAccount,
    #[account(constraint = a.address() != b.address() @ MyErr::BadConstraint)]
    pub b: UncheckedAccount,
}

// 9. executable
#[derive(Accounts)]
pub struct CheckExecutable {
    #[account(executable)]
    pub prog: UncheckedAccount,
}

// 10. rent_exempt = enforce
//
// Triggering the violation at runtime requires constructing an account
// the derive will accept through `load` but that is underfunded. For
// `UncheckedAccount` `load` is a no-op, so we can seed an arbitrary
// account via `LiteSVM::set_account` with data_len > 0 and lamports
// below the rent floor. See `tests/constraints.rs :: rent_exempt_*`.
#[derive(Accounts)]
pub struct CheckRentExempt {
    #[account(rent_exempt = enforce)]
    pub target: UncheckedAccount,
}

// 11. close = receiver
#[derive(Accounts)]
pub struct DoClose {
    #[account(mut, seeds = [b"data"], bump, close = receiver)]
    pub data: Account<Data>,
    #[account(mut)]
    pub receiver: UncheckedAccount,
}

// 12. seeds::program = OTHER_PROGRAM
#[derive(Accounts)]
pub struct CheckSeedsProgram {
    #[account(seeds = [b"other"], bump, seeds::program = OTHER_PROGRAM)]
    pub pda: UncheckedAccount,
}

// 13. init_if_needed
#[derive(Accounts)]
pub struct DoInitIfNeeded {
    #[account(mut)]
    pub payer: Signer,
    #[account(
        init_if_needed,
        payer = payer,
        seeds = [b"maybe"],
        bump,
    )]
    pub data: Account<Data>,
    pub system_program: Program<System>,
}

// 14. zeroed
#[derive(Accounts)]
pub struct CheckZeroed {
    #[account(zeroed)]
    pub data: Account<Data>,
}

// 15. signer on UncheckedAccount
#[derive(Accounts)]
pub struct CheckSigner {
    #[account(signer)]
    pub user: UncheckedAccount,
}

// Dead-code-eliminated instruction — exists to exercise the codegen for
// `rent_exempt = enforce` so any future regression in the derive path
// shows up at compile time even though the runtime-violation test for
// this constraint is parked (see `CheckRentExempt` doc comment).
#[allow(dead_code)]
fn _rent_exempt_codegen_witness(_ctx: &mut Context<CheckRentExempt>) {}

// -- IDL-only fixtures -------------------------------------------------------
//
// The structs below exist purely to exercise IDL emission — they are not
// wired into `#[program]`, so they contribute no runtime surface. They
// cover spellings the runtime fixtures above don't need (or don't reach):
//
//   - `address = <dotted field path>` — IDL-only here; the runtime
//     fixture `CheckAddress` uses a const RHS.
//   - `init_if_needed` without `seeds` — in the IDL this path sets
//     `signer:true` on the fresh-keypair account (there is no PDA to
//     derive). The runtime `DoInitIfNeeded` uses `seeds = [b"maybe"]`, so
//     a dedicated struct is needed to hit the no-seeds branch.

/// Fixture for the `address = <dotted path>` IDL test. The RHS walks a
/// sibling account's field, which clients resolve at request time.
#[derive(Accounts)]
pub struct CheckAddressFieldPath {
    pub data: Account<Data>,
    #[account(address = data.authority)]
    pub authority: UncheckedAccount,
}

/// Fixture for the `init_if_needed` no-seeds signer emission. The account
/// is a fresh keypair rather than a PDA, so the IDL marks it `signer:true`.
#[derive(Accounts)]
pub struct InitIfNeededNoSeeds {
    #[account(mut)]
    pub payer: Signer,
    #[account(init_if_needed, payer = payer)]
    pub data: Account<Data>,
    pub system_program: Program<System>,
}

// -- IDL emission tests ------------------------------------------------------
//
// Tests are parsed into the typed `anchor-lang-idl-spec` structs and
// asserted structurally — no substring matching — so a shape change in
// the derive's emitter fails the test at the impacted field rather than
// with a "got {full-json-blob}" dump.
//
// Only constraints that influence the IDL surface get tests here:
//   - `address = <expr>`        → `IdlInstructionAccount.address`
//   - `has_one = <field>`       → `IdlInstructionAccount.relations` (on the target)
//   - `init_if_needed` w/o seeds → `IdlInstructionAccount.signer`
//
// Constraints considered and intentionally skipped (runtime-only, no IDL
// surface): `zeroed`, `owner`, `constraint`, `executable`, `rent_exempt`,
// `close`. `seeds` / `seeds::program` are already covered structurally
// by the `seeds` program fixture; adding a duplicate here would only
// retest the shared `classify_seed` path.
#[cfg(all(test, feature = "idl-build"))]
mod idl_tests {
    use {
        super::*,
        anchor_lang_idl_spec::{IdlInstructionAccount, IdlInstructionAccountItem},
    };

    /// Parse an `__idl_accounts()` JSON blob into the typed spec enum.
    /// Panics with the raw JSON on parse failure so regressions in the
    /// emitter surface as readable diagnostics.
    fn parse(json: &str) -> Vec<IdlInstructionAccountItem> {
        serde_json::from_str(json)
            .unwrap_or_else(|e| panic!("failed to parse accounts JSON: {e}\njson: {json}"))
    }

    /// Unwrap a `Single` account at the given index or panic. `Composite`
    /// would only appear for a `Nested<Inner>` field, which none of the
    /// fixtures below use.
    fn single(items: &[IdlInstructionAccountItem], idx: usize) -> &IdlInstructionAccount {
        match &items[idx] {
            IdlInstructionAccountItem::Single(a) => a,
            IdlInstructionAccountItem::Composite(_) => {
                panic!("expected Single at index {idx}, got Composite")
            }
        }
    }

    mod idl_address {
        use super::*;

        #[test]
        fn const_path_rhs_emits_address_verbatim() {
            let items = parse(&CheckAddress::__idl_accounts());
            let account = single(&items, 0);
            assert_eq!(account.name, "pinned");
            assert_eq!(account.address.as_deref(), Some("PINNED_ADDRESS"));
        }

        #[test]
        fn const_path_rhs_with_custom_err_emits_address_verbatim() {
            // The `@ MyErr::BadAddress` error override is a runtime
            // concern; the IDL output for this spelling must match the
            // plain `address = PINNED_ADDRESS` variant exactly.
            let items = parse(&CheckAddressCustomErr::__idl_accounts());
            let account = single(&items, 0);
            assert_eq!(account.name, "pinned");
            assert_eq!(account.address.as_deref(), Some("PINNED_ADDRESS"));
        }

        #[test]
        fn dotted_field_path_rhs_emits_dotted_address() {
            // `address = data.authority` — the v2 replacement for
            // `has_one = authority` on the `data` field. The dotted
            // path is a client-side resolution instruction, not a
            // compile-time pubkey.
            let items = parse(&CheckAddressFieldPath::__idl_accounts());
            // `data` (the source account) carries no `address`.
            let data = single(&items, 0);
            assert_eq!(data.name, "data");
            assert_eq!(data.address, None);
            // `authority` (the referencing field) carries the dotted path.
            let authority = single(&items, 1);
            assert_eq!(authority.name, "authority");
            assert_eq!(authority.address.as_deref(), Some("data.authority"));
        }
    }

    mod idl_has_one {
        use super::*;

        #[test]
        fn has_one_emits_relation_on_target_field() {
            // `has_one = authority` on the `data` field produces an
            // inverse mapping: the `authority` account lists `data` in
            // its `relations[]`. Clients use this to know which sibling
            // to look up the authority pubkey on.
            let items = parse(&CheckHasOne::__idl_accounts());
            let data = single(&items, 0);
            assert_eq!(data.name, "data");
            // The source of the relation carries no `relations` entry.
            assert!(
                data.relations.is_empty(),
                "expected no relations on source field `data`, got {:?}",
                data.relations
            );
            let authority = single(&items, 1);
            assert_eq!(authority.name, "authority");
            assert_eq!(
                authority.relations,
                vec!["data".to_string()],
                "expected `relations:[\"data\"]` on the has_one target",
            );
        }

        #[test]
        fn has_one_with_custom_err_emits_same_relation() {
            // The `@ MyErr::BadAuthority` override is a runtime-only
            // concern; it must not change the IDL shape.
            let items = parse(&CheckHasOneCustomErr::__idl_accounts());
            let authority = single(&items, 1);
            assert_eq!(authority.name, "authority");
            assert_eq!(authority.relations, vec!["data".to_string()]);
        }
    }

    mod idl_init_if_needed {
        use super::*;

        #[test]
        fn init_if_needed_without_seeds_marks_account_as_signer() {
            // With no `seeds`, the fresh account must be signed by a
            // keypair the client generates at request time. The IDL
            // surfaces that as `signer:true` so the client builder
            // knows to add the signature.
            let items = parse(&InitIfNeededNoSeeds::__idl_accounts());
            let data = single(&items, 1);
            assert_eq!(data.name, "data");
            assert!(
                data.signer,
                "expected `signer:true` on fresh-keypair init_if_needed account",
            );
            assert!(
                data.pda.is_none(),
                "expected no `pda` entry on seedless init_if_needed account",
            );
        }

        #[test]
        fn init_if_needed_with_seeds_does_not_mark_signer() {
            // Control case: when `seeds` is present the account is a
            // PDA, and PDAs cannot sign. The IDL must leave `signer`
            // unset so the client doesn't attempt to add a signature.
            let items = parse(&DoInitIfNeeded::__idl_accounts());
            let data = single(&items, 1);
            assert_eq!(data.name, "data");
            assert!(
                !data.signer,
                "expected `signer:false` on PDA init_if_needed account",
            );
            assert!(
                data.pda.is_some(),
                "expected `pda` entry to be populated from seeds",
            );
        }
    }
}
