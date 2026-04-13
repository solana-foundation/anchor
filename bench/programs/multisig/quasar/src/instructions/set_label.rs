use {
    crate::state::MultisigConfig,
    quasar_lang::{prelude::*, sysvars::Sysvar as _},
};

#[derive(Accounts)]
pub struct SetLabel {
    #[account(mut)]
    pub creator: Signer,
    #[account(
        mut,
        has_one = creator,
        seeds = MultisigConfig::seeds(creator),
        bump = config.bump
    )]
    pub config: Account<MultisigConfig>,
    pub system_program: Program<System>,
}

impl SetLabel {
    #[inline(always)]
    pub fn update_label(&mut self, label: &str) -> Result<(), ProgramError> {
        let rent = Rent::get()?;
        self.config.set_label(
            label,
            self.creator.to_account_view(),
            rent.lamports_per_byte(),
            rent.exemption_threshold_raw(),
        )
    }
}
