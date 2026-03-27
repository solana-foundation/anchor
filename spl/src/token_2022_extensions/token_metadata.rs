// Avoiding AccountView deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_view::AccountView;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

use spl_pod::optional_keys::OptionalNonZeroPubkey;
use spl_token_metadata_interface::state::Field;

#[allow(unreachable_code, unused_variables, clippy::let_unit_value)]
pub fn token_metadata_initialize(
    ctx: CpiContext<'_, '_, TokenMetadataInitialize>,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    let ix = todo!();
    // ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
    Ok(())
}

#[derive(Accounts)]
pub struct TokenMetadataInitialize {
    pub program_id: AccountView,
    pub metadata: AccountView,
    pub update_authority: AccountView,
    pub mint_authority: AccountView,
    pub mint: AccountView,
}

#[allow(unreachable_code, unused_variables, clippy::let_unit_value)]
pub fn token_metadata_update_authority(
    ctx: CpiContext<'_, '_, TokenMetadataUpdateAuthority>,
    new_authority: OptionalNonZeroPubkey,
) -> Result<()> {
    let ix = todo!();
    // ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
    Ok(())
}

#[derive(Accounts)]
pub struct TokenMetadataUpdateAuthority {
    pub program_id: AccountView,
    pub metadata: AccountView,
    pub current_authority: AccountView,
    pub new_authority: AccountView,
}

#[allow(unreachable_code, unused_variables, clippy::let_unit_value)]
pub fn token_metadata_update_field(
    ctx: CpiContext<'_, '_, TokenMetadataUpdateField>,
    field: Field,
    value: String,
) -> Result<()> {
    let ix = todo!();
    // ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
    Ok(())
}

#[derive(Accounts)]
pub struct TokenMetadataUpdateField {
    pub program_id: AccountView,
    pub metadata: AccountView,
    pub update_authority: AccountView,
}
