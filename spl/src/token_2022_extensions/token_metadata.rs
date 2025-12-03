use anchor_lang::prelude::UncheckedAccount;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Result, ToAccountInfo};

use spl_pod::optional_keys::OptionalNonZeroPubkey;
use spl_token_metadata_interface::state::Field;

pub fn token_metadata_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, TokenMetadataInitialize<'info>>,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    let ix = spl_token_metadata_interface::instruction::initialize(
        ctx.accounts.program_id.key,
        ctx.accounts.metadata.key,
        ctx.accounts.update_authority.key,
        ctx.accounts.mint.key,
        ctx.accounts.mint_authority.key,
        name,
        symbol,
        uri,
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.program_id.to_account_info(),
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.update_authority.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenMetadataInitialize<'info> {
    pub program_id: UncheckedAccount<'info>,
    pub metadata: UncheckedAccount<'info>,
    pub update_authority: UncheckedAccount<'info>,
    pub mint_authority: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
}

pub fn token_metadata_update_authority<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, TokenMetadataUpdateAuthority<'info>>,
    new_authority: OptionalNonZeroPubkey,
) -> Result<()> {
    let ix = spl_token_metadata_interface::instruction::update_authority(
        ctx.accounts.program_id.key,
        ctx.accounts.metadata.key,
        ctx.accounts.current_authority.key,
        new_authority,
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.program_id.to_account_info(),
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.current_authority.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenMetadataUpdateAuthority<'info> {
    pub program_id: UncheckedAccount<'info>,
    pub metadata: UncheckedAccount<'info>,
    pub current_authority: UncheckedAccount<'info>,
    pub new_authority: UncheckedAccount<'info>,
}

pub fn token_metadata_update_field<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, TokenMetadataUpdateField<'info>>,
    field: Field,
    value: String,
) -> Result<()> {
    let ix = spl_token_metadata_interface::instruction::update_field(
        ctx.accounts.program_id.key,
        ctx.accounts.metadata.key,
        ctx.accounts.update_authority.key,
        field,
        value,
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.program_id.to_account_info(),
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.update_authority.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenMetadataUpdateField<'info> {
    pub program_id: UncheckedAccount<'info>,
    pub metadata: UncheckedAccount<'info>,
    pub update_authority: UncheckedAccount<'info>,
}
