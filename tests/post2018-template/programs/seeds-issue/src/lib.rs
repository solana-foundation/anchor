use anchor_lang::prelude::*;

declare_id!("5uYEr7j1eznfyH8tytwRakSMfy7gqNP647havHtSfuWt");

#[program]
pub mod seeds_issue {
    use super::*;

    /// GitHub issue #3947: Seeds referencing account field
    /// This fails during IDL generation with "cannot find value `user` in this scope"
    pub fn create_buyer(ctx: Context<CreateBuyer>) -> Result<()> {
        ctx.accounts.buyer.owner = ctx.accounts.user.key();
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateBuyer<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = 8 + 32,
        seeds = [b"buyer", user.key().as_ref()],
        bump
    )]
    pub buyer: Account<'info, BuyerInfo>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct BuyerInfo {
    pub owner: Pubkey,
}
