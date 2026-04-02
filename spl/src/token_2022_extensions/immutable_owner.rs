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

pub fn immutable_owner_initialize(ctx: CpiContext<'_, '_, ImmutableOwnerInitialize>) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_immutable_owner(
        *ctx.accounts.token_program_id.address(),
        *ctx.accounts.token_account.address(),
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.token_account],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct ImmutableOwnerInitialize {
    pub token_program_id: AccountView,
    pub token_account: AccountView,
}
