use anchor_lang::prelude::*;

declare_id!("848vVGVxp1kcRABrBid3uGCu1iiuPAHFuwpYfMMPPVZ7");

#[program]
mod log_compute_units {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("custom-compute: init, {} units", sol_remaining_compute_units());
        ctx.accounts.data.value = 42;
        Ok(())
    }

    pub fn update(ctx: Context<Update>, new_value: u64) -> Result<()> {
        msg!("custom-compute: update, {} units", sol_remaining_compute_units());
        ctx.accounts.data.value = new_value;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + 8
    )]
    pub data: Account<'info, Data>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut)]
    pub data: Account<'info, Data>,
}

#[account]
pub struct Data {
    pub value: u64,
}
