use anchor_lang::prelude::*;

// Define the program's instruction handlers.

#[program]
mod basic_2 {
    use super::*;

    pub fn create(ctx: Context<Create>, authority: Pubkey) -> ProgramResult {
        let counter = &mut ctx.accounts.counter;
        counter.authority = authority;
        counter.count = 0;
        Ok(())
    }

    pub fn increment(ctx: Context<Increment>) -> ProgramResult {
        let counter = &mut ctx.accounts.counter;
        counter.count += 1;
        Ok(())
    }
}

// Define the validated accounts for each handler.

#[derive(Accounts)]
pub struct Create<'info> {
    #[account(init)]
    pub counter: ProgramAccount<'info, Counter>,
}

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut, has_one = authority)]
    pub counter: ProgramAccount<'info, Counter>,
    #[account(signer)]
    pub authority: AccountInfo<'info>,
}

// Define the program owned accounts.

#[account]
pub struct Counter {
    pub authority: Pubkey,
    pub count: u64,
}
