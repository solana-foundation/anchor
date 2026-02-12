#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod test_instruction_validation {
    use super::*;

    // Nested Accounts structs - both parent and child have #[instruction]
    pub fn nested_both_instruction(
        _ctx: Context<NestedBothInstruction>,
        data: u64,
        value: u32,
    ) -> Result<()> {
        msg!("data: {}, value: {}", data, value);
        Ok(())
    }

    pub fn initialize_some_account(
        ctx: Context<InitializeSomeAccount>,
        data: u64,
    ) -> Result<()> {
        ctx.accounts.some_account.data = data;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(data: u64, value: u32)]
pub struct NestedBothInstruction<'info> {
    pub child: ChildWithInstruction<'info>,
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(data: u64, value: u32)]
pub struct ChildWithInstruction<'info> {
    pub some_account: Account<'info, SomeAccount>,
}

#[derive(Accounts)]
pub struct InitializeSomeAccount<'info> {
    #[account(init, payer = user, space = 8 + 8)]
    pub some_account: Account<'info, SomeAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct SomeAccount {
    pub data: u64,
}
