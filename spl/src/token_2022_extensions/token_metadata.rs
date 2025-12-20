// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{Result, Key};
use anchor_lang::{context::CpiContext, Accounts};

use spl_pod::optional_keys::OptionalNonZeroPubkey;
use spl_token_metadata_interface::state::Field;

pub fn token_metadata_initialize(
    ctx: CpiContext<'_, '_, TokenMetadataInitialize>,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenMetadataInitialize {
    pub program_id: AccountInfo,
    pub metadata: AccountInfo,
    pub update_authority: AccountInfo,
    pub mint_authority: AccountInfo,
    pub mint: AccountInfo,
}

pub fn token_metadata_update_authority(
    ctx: CpiContext<'_, '_, TokenMetadataUpdateAuthority>,
    new_authority: OptionalNonZeroPubkey,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenMetadataUpdateAuthority {
    pub program_id: AccountInfo,
    pub metadata: AccountInfo,
    pub current_authority: AccountInfo,
    pub new_authority: AccountInfo,
}

pub fn token_metadata_update_field(
    ctx: CpiContext<'_, '_, TokenMetadataUpdateField>,
    field: Field,
    value: String,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenMetadataUpdateField {
    pub program_id: AccountInfo,
    pub metadata: AccountInfo,
    pub update_authority: AccountInfo,
}
