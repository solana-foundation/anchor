use anchor_lang::prelude::*;

declare_id!("55cqWEVF1WMMBxzW6BPHTV2KGZ5NT9454sjfU58j2a4j");

#[program]
pub mod pda_payer {
    use super::*;

    pub fn init_with_pda_payer(ctx: Context<MyInstruction>) -> Result<()> {
        ctx.accounts.new_account.data = 42;
        Ok(())
    }
}

#[account]
pub struct MyData {
    pub data: u64,
}

#[derive(Accounts)]
pub struct MyInstruction<'info> {
    // Seeds must be declared here
    #[account(
        mut,
        seeds = [b"my-pda"],
        bump,
    )]
    pub pda_account: SystemAccount<'info>,

    // Then used as payer below
    #[account(
        init,
        payer = pda_account,  // PDA as payer
        space = 8 + 32,
    )]
    pub new_account: Account<'info, MyData>,

    pub system_program: Program<'info, System>,
}
