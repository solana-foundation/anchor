use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;

mod context;
mod state;

use context::*;
use state::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod bad_one {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize()
    }

    pub fn transfer_with_cpi_bad(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        ctx.accounts.transfer(amount)
    }
}
