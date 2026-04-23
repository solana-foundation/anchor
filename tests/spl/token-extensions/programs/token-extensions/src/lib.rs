//! An example of a program with token extensions enabled
//!
//! This program is intended to implement various token2022 extensions
//!
//! <https://spl.solana.com/token-2022/extensions>

use anchor_lang::prelude::*;

pub mod instructions;
pub mod utils;
pub use {instructions::*, utils::*};

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

    pub fn create_group_pointer_mint(_ctx: Context<CreateGroupPointerMint>) -> Result<()> {
        Ok(())
    }

    pub fn update_group_pointer(
        ctx: Context<UpdateGroupPointer>,
        new_group_address: Option<Pubkey>,
    ) -> Result<()> {
        instructions::update_group_pointer_handler(ctx, new_group_address)
    }

    pub fn enable_cpi_guard(ctx: Context<EnableCpiGuard>) -> Result<()> {
        instructions::enable_cpi_guard_handler(ctx)
    }

    pub fn disable_cpi_guard(ctx: Context<DisableCpiGuard>) -> Result<()> {
        instructions::disable_cpi_guard_handler(ctx)
    }

    pub fn check_toggle_pause(ctx: Context<CheckTogglePause>) -> Result<()> {
        instructions::toggle_pause_handler(ctx)
    }

    pub fn check_pausable_authority_constraint(
        ctx: Context<CheckPausableAuthorityConstraint>,
    ) -> Result<()> {
        instructions::check_pausable_authority_constraint_handler(ctx)
    }
}
