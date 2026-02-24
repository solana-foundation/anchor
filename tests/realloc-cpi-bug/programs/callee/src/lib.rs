use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod callee {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.data_account.data = vec![0];
        ctx.accounts.data_account.bump = ctx.bumps.data_account;
        Ok(())
    }

    pub fn realloc(ctx: Context<Realloc>, len: u16) -> Result<()> {
        ctx.accounts
            .data_account
            .data
            .resize_with(len as usize, Default::default);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        seeds = [b"data"],
        bump,
        space = DataAccount::space(1),
    )]
    pub data_account: Account<'info, DataAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(len: u16)]
pub struct Realloc<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"data"],
        bump = data_account.bump,
        realloc = DataAccount::space(len as usize),
        realloc::payer = authority,
        realloc::zero = false,
    )]
    pub data_account: Account<'info, DataAccount>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct DataAccount {
    pub data: Vec<u8>,
    pub bump: u8,
}

impl DataAccount {
    pub fn space(len: usize) -> usize {
        8 + (4 + len) + 1
    }
}
