use anchor_lang::prelude::*;

// Intentionally different program id than the one defined in Anchor.toml.
declare_id!("4D6rvpR7TSPwmFottLGa5gpzMcJ76kN8bimQHV9rogjH");

#[program]
pub mod duplicate_mutable_accounts {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, initial: u64) -> Result<()> {
        ctx.accounts.data_account.count = initial;
        Ok(())
    }

    // This one should FAIL if the same mutable account is passed twice
    // (Anchor disallows duplicate mutable accounts here).
    pub fn fails_duplicate_mutable(ctx: Context<FailsDuplicateMutable>) -> Result<()> {
        ctx.accounts.account1.count += 1;
        ctx.accounts.account2.count += 1;
        Ok(())
    }

    // This one should SUCCEED even if the same account is passed twice,
    // thanks to the `dup` constraint.
    pub fn allows_duplicate_mutable(ctx: Context<AllowsDuplicateMutable>) -> Result<()> {
        ctx.accounts.account1.count += 1;
        ctx.accounts.account2.count += 1;
        Ok(())
    }

    // Readonly duplicates should always be fine: we just read (no mutation).
    pub fn allows_duplicate_readonly(_ctx: Context<AllowsDuplicateReadonly>) -> Result<()> {
        Ok(())
    }
}

#[account]
pub struct Counter {
    pub count: u64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 8)]
    pub data_account: Account<'info, Counter>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// No extra accounts here, because tests only pass account1 and account2.
#[derive(Accounts)]
pub struct FailsDuplicateMutable<'info> {
    #[account(mut)]
    pub account1: Account<'info, Counter>,
    #[account(mut)]
    pub account2: Account<'info, Counter>,
}

// Allow the same mutable account to be supplied twice.
#[derive(Accounts)]
pub struct AllowsDuplicateMutable<'info> {
    #[account(mut)]
    pub account1: Account<'info, Counter>,
    #[account(mut, dup)]
    pub account2: Account<'info, Counter>,
}

// Readonly accounts (no `mut`), duplicates allowed by nature.
#[derive(Accounts)]
pub struct AllowsDuplicateReadonly<'info> {
    pub account1: Account<'info, Counter>,
    pub account2: Account<'info, Counter>,
}
