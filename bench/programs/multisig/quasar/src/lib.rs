#![no_std]

use quasar_lang::prelude::*;

mod instructions;
use instructions::*;
mod state;
#[cfg(test)]
mod tests;

declare_id!("44444444444444444444444444444444444444444444");

#[program]
mod quasar_multisig {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn create(ctx: CtxWithRemaining<Create>, threshold: u8) -> Result<(), ProgramError> {
        ctx.accounts
            .create_multisig(threshold, &ctx.bumps, ctx.remaining_accounts())
    }

    #[instruction(discriminator = 1)]
    pub fn deposit(ctx: Ctx<Deposit>, amount: u64) -> Result<(), ProgramError> {
        ctx.accounts.deposit(amount)
    }

    #[instruction(discriminator = 2)]
    pub fn set_label(ctx: Ctx<SetLabel>, label: String<32>) -> Result<(), ProgramError> {
        ctx.accounts.update_label(label)
    }

    #[instruction(discriminator = 3)]
    pub fn execute_transfer(
        ctx: CtxWithRemaining<ExecuteTransfer>,
        amount: u64,
    ) -> Result<(), ProgramError> {
        ctx.accounts
            .verify_and_transfer(amount, &ctx.bumps, ctx.remaining_accounts())
    }
}
