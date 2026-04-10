// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use {
    anchor_lang::{
        context::CpiContext,
        pinocchio_runtime::{account_info::AccountInfo, pubkey::Pubkey},
        Accounts, Result,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
    spl_token_metadata_interface::state::Field,
};

pub fn token_metadata_initialize(
    ctx: CpiContext<'_, '_, TokenMetadataInitialize>,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    let ix = spl_token_metadata_interface::instruction::initialize(
        ctx.accounts.program_id.address(),
        ctx.accounts.metadata.address(),
        ctx.accounts.update_authority.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.mint_authority.address(),
        name,
        symbol,
        uri,
    );
    crate::cpi_util::invoke_signed_solana_instruction(ix,
        &[
            ctx.accounts.program_id,
            ctx.accounts.metadata,
            ctx.accounts.update_authority,
            ctx.accounts.mint,
            ctx.accounts.mint_authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
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
    let ix = spl_token_metadata_interface::instruction::update_authority(
        ctx.accounts.program_id.address(),
        ctx.accounts.metadata.address(),
        ctx.accounts.current_authority.address(),
        new_authority,
    );
    crate::cpi_util::invoke_signed_solana_instruction(ix,
        &[
            ctx.accounts.program_id,
            ctx.accounts.metadata,
            ctx.accounts.current_authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
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
    let ix = spl_token_metadata_interface::instruction::update_field(
        ctx.accounts.program_id.address(),
        ctx.accounts.metadata.address(),
        ctx.accounts.update_authority.address(),
        field,
        value,
    );
    crate::cpi_util::invoke_signed_solana_instruction(ix,
        &[
            ctx.accounts.program_id,
            ctx.accounts.metadata,
            ctx.accounts.update_authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenMetadataUpdateField {
    pub program_id: AccountInfo,
    pub metadata: AccountInfo,
    pub update_authority: AccountInfo,
}
