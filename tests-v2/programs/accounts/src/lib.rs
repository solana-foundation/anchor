//! Test program exercising account-wrapper types that sit outside the
//! constraints/seeds/cpi suites — Sysvar, Box<Account>, SystemAccount,
//! and bare UncheckedAccount read paths.

use {
    anchor_lang_v2::prelude::*,
    pinocchio::sysvars::{clock::Clock, rent::Rent},
};

declare_id!("Acc1111111111111111111111111111111111111111");

#[account]
pub struct Counter {
    pub value: u64,
}

#[program]
pub mod accounts_test {
    use super::*;

    /// Initialize a counter. Exercises `BorshAccount`'s init path through a
    /// regular `Account<T>` — also kicks the `Box<Account<T>>` handler below
    /// into a known state.
    #[discrim = 0]
    pub fn initialize(ctx: &mut Context<Initialize>) -> Result<()> {
        ctx.accounts.counter.value = 1;
        Ok(())
    }

    /// Loads the counter inside a `Box` and mutates it through `Deref`.
    /// Hits `AnchorAccount for Box<T>` (`accounts/boxed.rs`).
    #[discrim = 1]
    pub fn bump_boxed(ctx: &mut Context<BumpBoxed>) -> Result<()> {
        ctx.accounts.counter.value = ctx.accounts.counter.value.wrapping_add(1);
        Ok(())
    }

    /// Reads the Clock sysvar. Exercises `Sysvar<Clock>::load` and `Deref`
    /// forwarding to the inner pinocchio type.
    #[discrim = 2]
    pub fn read_clock(ctx: &mut Context<ReadClock>) -> Result<()> {
        // Touch several Clock fields so the register trace covers the
        // deref/getter path.
        let clock = &*ctx.accounts.clock;
        let _ = clock.slot;
        let _ = clock.epoch;
        let _ = clock.unix_timestamp;
        Ok(())
    }

    /// Reads the Rent sysvar. Same rationale as `read_clock`.
    #[discrim = 3]
    pub fn read_rent(ctx: &mut Context<ReadRent>) -> Result<()> {
        let rent = &*ctx.accounts.rent;
        let _ = rent.try_minimum_balance(100);
        Ok(())
    }

    /// Take a `SystemAccount` — validates the passed account is owned by
    /// the System program. Exercises `accounts/system_account.rs`.
    #[discrim = 4]
    pub fn check_system(ctx: &mut Context<CheckSystem>) -> Result<()> {
        let _ = ctx.accounts.wallet.address();
        Ok(())
    }

    /// Read-only UncheckedAccount — exercises load + accessor paths on
    /// `accounts/unchecked_account.rs`.
    #[discrim = 5]
    pub fn touch_unchecked(ctx: &mut Context<TouchUnchecked>) -> Result<()> {
        let _ = ctx.accounts.any_account.address();
        Ok(())
    }
}

// -- Accounts structs --------------------------------------------------------

#[derive(Accounts)]
pub struct Initialize {
    #[account(mut)]
    pub payer: Signer,
    #[account(init, payer = payer, seeds = [b"counter"], bump)]
    pub counter: Account<Counter>,
    pub system_program: Program<System>,
}

#[derive(Accounts)]
pub struct BumpBoxed {
    #[account(mut)]
    pub counter: Box<Account<Counter>>,
}

#[derive(Accounts)]
pub struct ReadClock {
    pub clock: Sysvar<Clock>,
}

#[derive(Accounts)]
pub struct ReadRent {
    pub rent: Sysvar<Rent>,
}

#[derive(Accounts)]
pub struct CheckSystem {
    pub wallet: SystemAccount,
}

#[derive(Accounts)]
pub struct TouchUnchecked {
    pub any_account: UncheckedAccount,
}
