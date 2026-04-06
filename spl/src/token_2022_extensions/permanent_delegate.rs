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

pub fn permanent_delegate_initialize(
    ctx: CpiContext<'_, '_, PermanentDelegateInitialize>,
    permanent_delegate: &Pubkey,
) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_permanent_delegate(
        *ctx.accounts.token_program_id.address(),
        *ctx.accounts.mint.address(),
        permanent_delegate,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct PermanentDelegateInitialize {
    pub token_program_id: AccountView,
    pub mint: AccountView,
}
