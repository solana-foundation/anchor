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
