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

pub fn transfer_hook_initialize(
    ctx: CpiContext<'_, '_, TransferHookInitialize>,
    authority: Option<Pubkey>,
    transfer_hook_program_id: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::transfer_hook::instruction::initialize(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        authority,
        transfer_hook_program_id,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct TransferHookInitialize {
    pub token_program_id: AccountView,
    pub mint: AccountView,
}

pub fn transfer_hook_update(
    ctx: CpiContext<'_, '_, TransferHookUpdate>,
    transfer_hook_program_id: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::transfer_hook::instruction::update(
        *ctx.accounts.token_program_id.address(),
        *ctx.accounts.mint.address(),
        *ctx.accounts.authority.address(),
        &[],
        transfer_hook_program_id,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.mint,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct TransferHookUpdate {
    pub token_program_id: AccountView,
    pub mint: AccountView,
    pub authority: AccountView,
}
