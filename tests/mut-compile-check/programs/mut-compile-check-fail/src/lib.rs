use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[account]
pub struct MyData {
    pub value: u64,
}

#[derive(Accounts)]
pub struct MutateWithoutMut<'info> {
    // NO #[account(mut)] here
    pub data: Account<'info, MyData>,
}

#[program]
pub mod mut_compile_check_fail {
    use super::*;

    pub fn mutate_without_mut(ctx: Context<MutateWithoutMut>, new_value: u64) -> Result<()> {
        // ERROR: This should fail at compile time
        ctx.accounts.data.value = new_value;
        Ok(())
    }

    pub fn mutate_without_mut_deref(ctx: Context<MutateWithoutMut>, new_value: u64) -> Result<()> {
        // ERROR: This should fail at compile time
        let data = &mut ctx.accounts.data;
        data.value = new_value;
        Ok(())
    }

    pub fn mutate_without_mut_set_inner(ctx: Context<MutateWithoutMut>, new_value: u64) -> Result<()> {
        // ERROR: This should fail at compile time
        ctx.accounts.data.set_inner(MyData { value: new_value });
        Ok(())
    }
}

