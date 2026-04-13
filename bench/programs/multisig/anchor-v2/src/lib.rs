use anchor_lang_v2::prelude::*;

mod errors;
mod instructions;
mod state;

pub use errors::*;
use instructions::*;
pub use state::*;

declare_id!("44444444444444444444444444444444444444444444");

#[program]
pub mod multisig_v2 {
    use super::*;

    pub fn create(ctx: &mut Context<'_, Create>, threshold: u8) -> Result<()> {
        let remaining = ctx.remaining_accounts;
        ctx.accounts.create_multisig(threshold, remaining)?;
        ctx.accounts.config.bump = ctx.bumps.config;
        Ok(())
    }

    pub fn deposit(ctx: &mut Context<'_, Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)
    }

    pub fn set_label(ctx: &mut Context<'_, SetLabel>, label_len: u8, label: [u8; 32]) -> Result<()> {
        ctx.accounts.update_label(label_len, label)
    }

    pub fn execute_transfer(ctx: &mut Context<'_, ExecuteTransfer>, amount: u64) -> Result<()> {
        let remaining = ctx.remaining_accounts;
        ctx.accounts.verify_and_transfer(amount, ctx.bumps.vault, remaining)
    }
}
