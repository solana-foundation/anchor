//! An example of a program with token extensions enabled
//!
//! This program is intended to implement various token2022 extensions
//!
//! <https://spl.solana.com/token-2022/extensions>

use anchor_lang::prelude::*;

pub mod instructions;
pub mod utils;
pub use instructions::*;
pub use utils::*;

declare_id!("tKEkkQtgMXhdaz5NMTR3XbdUu215sZyHSj6Menvous1");

#[program]
pub mod token_extensions {
    use super::*;

    pub fn create_mint_account(
        ctx: Context<CreateMintAccount>,
        args: CreateMintAccountArgs,
    ) -> Result<()> {
        instructions::handler(ctx, args)
    }

    pub fn check_mint_extensions_constraints(
        _ctx: Context<CheckMintExtensionConstraints>,
    ) -> Result<()> {
        Ok(())
    }
}
