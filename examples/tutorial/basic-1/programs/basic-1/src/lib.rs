use anchor_lang::prelude::*;

#[program]
mod basic_1 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, data: u64) -> ProgramResult {
        let my_account = &mut ctx.accounts.my_account;
        my_account.data = data;
        Ok(())
    }

    pub fn update(ctx: Context<Update>, data: u64) -> ProgramResult {
        let my_account = &mut ctx.accounts.my_account;
        my_account.data = data;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init)]
    pub my_account: ProgramAccount<'info, MyAccount>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut)]
    pub my_account: ProgramAccount<'info, MyAccount>,
}

#[account]
pub struct MyAccount {
    pub data: u64,
}
