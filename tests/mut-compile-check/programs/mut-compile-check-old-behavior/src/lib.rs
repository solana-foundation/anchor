use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[account]
pub struct MyData {
    pub value: u64,
}

#[derive(Accounts)]
pub struct OldBehavior<'info> {
    pub data: Account<'info, MyData>,
}

#[program]
pub mod old_behavior {
    use super::*;

    pub fn mutate_without_mut_old_behavior(
        ctx: Context<OldBehavior>,
        new_value: u64,
    ) -> Result<()> {
        // This would have compiled in Anchor 0.32 but failed silently at runtime
        ctx.accounts.data.value = new_value;
        Ok(())
    }
}
