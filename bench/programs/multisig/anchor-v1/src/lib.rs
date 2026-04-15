use anchor_lang::prelude::*;

mod instructions;
use instructions::*;
mod state;
pub use state::*;

declare_id!("44444444444444444444444444444444444444444444");

#[program]
pub mod multisig_v1 {
    use super::*;

    pub fn create<'info>(ctx: Context<'info, Create<'info>>, threshold: u8) -> Result<()> {
        let remaining = ctx.remaining_accounts.to_vec();
        ctx.accounts
            .create_multisig(threshold, &ctx.bumps, &remaining)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)
    }

    pub fn set_label(ctx: Context<SetLabel>, label: String) -> Result<()> {
        ctx.accounts.update_label(label.as_str())
    }

    pub fn execute_transfer<'info>(
        ctx: Context<'info, ExecuteTransfer<'info>>,
        amount: u64,
    ) -> Result<()> {
        let remaining = ctx.remaining_accounts.to_vec();
        ctx.accounts
            .verify_and_transfer(amount, &ctx.bumps, &remaining)
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("At least one signer is required and the threshold cannot exceed signer count.")]
    InvalidThreshold,
    #[msg("Too many signers were provided.")]
    TooManySigners,
    #[msg("A required signer was missing.")]
    MissingRequiredSignature,
    #[msg("The label exceeds the maximum supported length.")]
    LabelTooLong,
    #[msg("Only the creator may update the multisig label.")]
    UnauthorizedCreator,
}
