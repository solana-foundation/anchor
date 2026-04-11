use {crate::state::MultisigConfig, anchor_lang::prelude::*};

#[derive(Accounts)]
pub struct SetLabel<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        mut,
        has_one = creator,
        seeds = [b"multisig", creator.key().as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, MultisigConfig>,
    pub system_program: Program<'info, System>,
}

impl<'info> SetLabel<'info> {
    #[inline(always)]
    pub fn update_label(&mut self, label: &str) -> Result<()> {
        self.config.set_label(&self.creator, label)
    }
}
