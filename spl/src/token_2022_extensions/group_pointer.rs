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

pub fn group_pointer_initialize(
    ctx: CpiContext<'_, '_, GroupPointerInitialize>,
    authority: Option<Pubkey>,
    group_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::group_pointer::instruction::initialize(
        *ctx.accounts.token_program_id.address(),
        *ctx.accounts.mint.address(),
        authority,
        group_address,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupPointerInitialize {
    pub token_program_id: AccountView,
    pub mint: AccountView,
}

pub fn group_pointer_update(
    ctx: CpiContext<'_, '_, GroupPointerUpdate>,
    group_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::group_pointer::instruction::update(
        *ctx.accounts.token_program_id.address(),
        *ctx.accounts.mint.address(),
        *ctx.accounts.authority.address(),
        &[*ctx.accounts.authority.address()],
        group_address,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupPointerUpdate {
    pub token_program_id: AccountView,
    pub mint: AccountView,
    pub authority: AccountView,
}
