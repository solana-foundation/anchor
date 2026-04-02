// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use {
    anchor_lang::{
        context::CpiContext,
        pinocchio_runtime::{account_view::AccountView, pubkey::Pubkey},
        Accounts, Result,
    },
    spl_token_2022_interface as spl_token_2022,
};

pub fn mint_close_authority_initialize(
    ctx: CpiContext<'_, '_, MintCloseAuthorityInitialize>,
    authority: Option<&Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_mint_close_authority(
        *ctx.accounts.token_program_id.address(),
        *ctx.accounts.mint.address(),
        authority,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct MintCloseAuthorityInitialize {
    pub token_program_id: AccountView,
    pub mint: AccountView,
}
