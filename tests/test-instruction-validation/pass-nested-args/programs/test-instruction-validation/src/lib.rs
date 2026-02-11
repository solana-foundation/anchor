#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod test_instruction_validation {
    use super::*;

    // Test: Nested Accounts structs - parent has #[instruction], child doesn't
    // This should work because only parent deserializes instruction args
    pub fn nested_args_works(
        _ctx: Context<NestedArgsWorks>,
        data: u64,
        value: u32,
    ) -> Result<()> {
        msg!("data: {}, value: {}", data, value);
        Ok(())
    }

    // Test: Nested Accounts structs - parent has #[instruction] with partial args
    // Child struct doesn't have #[instruction], so it doesn't try to deserialize
    pub fn nested_partial_args(
        _ctx: Context<NestedPartialArgs>,
        a: u64,
        b: u32,
        c: u64,
        d: u8,
    ) -> Result<()> {
        msg!("a: {}, b: {}, c: {}, d: {}", a, b, c, d);
        Ok(())
    }

    // Test: Nested Accounts structs - both parent and child have #[instruction]
    // This compiles but will fail at runtime due to buffer consumption issue (GitHub #2942)
    // Our validation should still work correctly at compile time
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
pub struct NestedArgsWorks<'info> {
    pub child: ChildAccount<'info>,
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct ChildAccount<'info> {
    pub some_account: Account<'info, SomeAccount>,
}

#[derive(Accounts)]
#[instruction(b: u32, d: u8)]
pub struct NestedPartialArgs<'info> {
    pub child: ChildAccountNoInstruction<'info>,
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct ChildAccountNoInstruction<'info> {
    pub some_account: Account<'info, SomeAccount>,
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
