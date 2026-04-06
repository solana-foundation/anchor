// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use {
    anchor_lang::{
        context::CpiContext,
        pinocchio_runtime::{account_view::AccountView, pubkey::Pubkey},
        Accounts, Key, Result,
    },
    spl_token_2022_interface as spl_token_2022,
};

pub fn metadata_pointer_initialize(
    ctx: CpiContext<'_, '_, MetadataPointerInitialize>,
    authority: Option<Pubkey>,
    metadata_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::metadata_pointer::instruction::initialize(
        *ctx.accounts.token_program_id.address(),
        *ctx.accounts.mint.address(),
        authority,
        metadata_address,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct MetadataPointerInitialize {
    pub token_program_id: AccountView,
    pub mint: AccountView,
}
