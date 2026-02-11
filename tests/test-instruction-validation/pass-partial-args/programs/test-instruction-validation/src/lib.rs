#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod test_instruction_validation {
    use super::*;

    pub fn partial_args(
        _ctx: Context<PartialArgs>,
        a: u64,
        b: u32,
        c: u64,
        d: u8,
    ) -> Result<()> {
        msg!("a: {}, b: {}, c: {}, d: {}", a, b, c, d);
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(d: u8, b: u32)] 
pub struct PartialArgs<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
}