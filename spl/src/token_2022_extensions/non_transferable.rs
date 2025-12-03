use anchor_lang::prelude::UncheckedAccount;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Result, ToAccountInfo};
use spl_token_2022_interface as spl_token_2022;

pub fn non_transferable_mint_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, NonTransferableMintInitialize<'info>>,
) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_non_transferable_mint(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
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
pub struct NonTransferableMintInitialize<'info> {
    pub token_program_id: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
}
