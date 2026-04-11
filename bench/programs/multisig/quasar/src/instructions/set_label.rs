use {crate::state::MultisigConfig, quasar_lang::prelude::*};

#[derive(Accounts)]
pub struct SetLabel<'info> {
    pub creator: &'info mut Signer,
    #[account(
        mut,
        has_one = creator,
        seeds = MultisigConfig::seeds(creator),
        bump = config.bump
    )]
    pub config: Account<MultisigConfig<'info>>,
    pub system_program: &'info Program<System>,
}

impl<'info> SetLabel<'info> {
    #[inline(always)]
    pub fn update_label(&mut self, label: &str) -> Result<(), ProgramError> {
        self.config.set_label(self.creator, label)
    }
}
