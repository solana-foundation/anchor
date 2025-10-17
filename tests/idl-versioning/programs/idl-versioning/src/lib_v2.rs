use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111111");

#[program]
pub mod idl_versioning {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, data: u64) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count = data;
        counter.authority = ctx.accounts.user.key();
        counter.last_updated = Clock::get()?.unix_timestamp;
        msg!("Counter initialized with value: {}", data);
        Ok(())
    }

    pub fn increment(ctx: Context<Update>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count = counter.count.checked_add(1).unwrap();
        counter.last_updated = Clock::get()?.unix_timestamp;
        msg!("Counter incremented to: {}", counter.count);
        Ok(())
    }

    /// New function added in v2
    pub fn decrement(ctx: Context<Update>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count = counter.count.checked_sub(1).unwrap();
        counter.last_updated = Clock::get()?.unix_timestamp;
        msg!("Counter decremented to: {}", counter.count);
        Ok(())
    }

    /// New function added in v2
    pub fn reset(ctx: Context<Update>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count = 0;
        counter.last_updated = Clock::get()?.unix_timestamp;
        msg!("Counter reset to 0");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 8 + 32 + 8)]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[account]
pub struct Counter {
    pub count: u64,
    pub authority: Pubkey,
    /// New field added in v2
    pub last_updated: i64,
}
