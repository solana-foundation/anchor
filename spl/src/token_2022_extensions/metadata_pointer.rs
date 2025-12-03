use anchor_lang::prelude::UncheckedAccount;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Result, ToAccountInfo};
use spl_token_2022_interface as spl_token_2022;

pub fn metadata_pointer_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, MetadataPointerInitialize<'info>>,
    authority: Option<Pubkey>,
    metadata_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::metadata_pointer::instruction::initialize(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        authority,
        metadata_address,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id.to_account_info(),
            ctx.accounts.mint.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct MetadataPointerInitialize<'info> {
    pub token_program_id: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
}
