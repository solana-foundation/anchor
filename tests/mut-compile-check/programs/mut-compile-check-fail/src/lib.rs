//! Test program that SHOULD FAIL to compile.
//!
//! This program attempts to mutate a ReadOnlyAccount, which should
//! cause compile-time errors since ReadOnlyAccount doesn't implement DerefMut.
use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[account]
pub struct MyData {
    pub value: u64,
}

/// Accounts struct using ReadOnlyAccount - mutation is NOT allowed
#[derive(Accounts)]
pub struct MutateWithoutMut<'info> {
    // Using ReadOnlyAccount - mutation should fail at compile time
    pub data: ReadOnlyAccount<'info, MyData>,
}

#[program]
pub mod mut_compile_check_fail {
    use super::*;

    /// ERROR: This should fail at compile time
    /// ReadOnlyAccount doesn't implement DerefMut
    pub fn mutate_without_mut(ctx: Context<MutateWithoutMut>, new_value: u64) -> Result<()> {
        // This line should cause: error[E0594]: cannot assign to data in dereference of `ReadOnlyAccount`
        ctx.accounts.data.value = new_value;
        Ok(())
    }

    /// ERROR: This should also fail at compile time
    pub fn mutate_without_mut_deref(ctx: Context<MutateWithoutMut>, new_value: u64) -> Result<()> {
        // This line should cause a compile error - can't get &mut from ReadOnlyAccount
        let data = &mut ctx.accounts.data;
        data.value = new_value;
        Ok(())
    }

    /// ERROR: This should fail at compile time
    /// ReadOnlyAccount doesn't have set_inner method
    pub fn mutate_without_mut_set_inner(
        ctx: Context<MutateWithoutMut>,
        new_value: u64,
    ) -> Result<()> {
        // This line should cause: error[E0599]: no method named `set_inner` found
        ctx.accounts.data.set_inner(MyData { value: new_value });
        Ok(())
    }
}
