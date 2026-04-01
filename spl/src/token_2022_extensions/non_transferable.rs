// Avoiding AccountView deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::{
    context::CpiContext,
    pinocchio_runtime::{account_view::AccountView, pubkey::Pubkey},
    Accounts, Key, Result,
};

#[allow(unreachable_code, unused_variables, clippy::let_unit_value)]
pub fn non_transferable_mint_initialize(
    ctx: CpiContext<'_, '_, NonTransferableMintInitialize>,
) -> Result<()> {
    let ix = todo!();
    // ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
    Ok(())
}

#[derive(Accounts)]
pub struct NonTransferableMintInitialize {
    pub token_program_id: AccountView,
    pub mint: AccountView,
}
