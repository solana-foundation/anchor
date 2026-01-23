//! Test program to verify compile-time checking of account mutations.
//!
//! This program demonstrates correct usage:
//! - Account with #[account(mut)] allows mutation
//! - ReadOnlyAccount allows reading but not mutation
use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod mut_compile_check {
    use super::*;

    /// Mutate account data - allowed because #[account(mut)] is specified
    pub fn mutate_with_mut(ctx: Context<MutateWithMut>, new_value: u64) -> Result<()> {
        ctx.accounts.data.value = new_value;
        Ok(())
    }

    /// Alternative mutation via mutable reference - allowed with #[account(mut)]
    pub fn mutate_with_mut_alternative(ctx: Context<MutateWithMut>, new_value: u64) -> Result<()> {
        let data = &mut ctx.accounts.data;
        data.value = new_value;
        Ok(())
    }

    /// Mutation via set_inner - allowed with #[account(mut)]
    pub fn mutate_with_mut_set_inner(ctx: Context<MutateWithMut>, new_value: u64) -> Result<()> {
        ctx.accounts.data.set_inner(MyData { value: new_value });
        Ok(())
    }

    /// Read from ReadOnlyAccount - allowed
    pub fn read_only(ctx: Context<ReadOnlyCtx>) -> Result<()> {
        msg!("Current value: {}", ctx.accounts.data.value);
        Ok(())
    }

    /// Mutate ReadOnlyAccount - should fail at compile time
    pub fn mutate_readonly(ctx: Context<ReadOnlyCtx>, new_value: u64) -> Result<()> {
        ctx.accounts.data.value = new_value;
        Ok(())
    }
}

#[account]
pub struct MyData {
    pub value: u64,
}

/// Accounts struct with mutable account - allows mutation
#[derive(Accounts)]
pub struct MutateWithMut<'info> {
    #[account(mut)]
    pub data: Account<'info, MyData>,
}

/// Accounts struct with ReadOnlyAccount - prevents mutation at compile time
#[derive(Accounts)]
pub struct ReadOnlyCtx<'info> {
    pub data: ReadOnlyAccount<'info, MyData>,
}
