//! Test program to verify compile-time checking of account mutations.
use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod mut_compile_check {
    use super::*;

    pub fn mutate_with_mut(ctx: Context<MutateWithMut>, new_value: u64) -> Result<()> {
        ctx.accounts.data.value = new_value;
        Ok(())
    }

    pub fn mutate_with_mut_alternative(ctx: Context<MutateWithMut>, new_value: u64) -> Result<()> {
        let data = &mut ctx.accounts.data;
        data.value = new_value;
        Ok(())
    }

    pub fn mutate_with_mut_set_inner(ctx: Context<MutateWithMut>, new_value: u64) -> Result<()> {
        ctx.accounts.data.set_inner(MyData { value: new_value });
        Ok(())
    }
}

#[account]
pub struct MyData {
    pub value: u64,
}

#[derive(Accounts)]
pub struct MutateWithMut<'info> {
    #[account(mut)]
    pub data: Account<'info, MyData>,
}

