// Avoiding AccountView deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_view::AccountView;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

#[allow(unreachable_code, unused_variables, clippy::let_unit_value)]
pub fn token_group_initialize(
    ctx: CpiContext<'_, '_, TokenGroupInitialize>,
    update_authority: Option<Pubkey>,
    max_size: u64,
) -> Result<()> {
    let ix = todo!();
    // ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
    Ok(())
}

#[derive(Accounts)]
pub struct TokenGroupInitialize {
    pub program_id: AccountView,
    pub group: AccountView,
    pub mint: AccountView,
    pub mint_authority: AccountView,
}

#[allow(unreachable_code, unused_variables, clippy::let_unit_value)]
pub fn token_member_initialize(ctx: CpiContext<'_, '_, TokenMemberInitialize>) -> Result<()> {
    let ix = todo!();
    // ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
    Ok(())
}

#[derive(Accounts)]
pub struct TokenMemberInitialize {
    pub program_id: AccountView,
    pub member: AccountView,
    pub member_mint: AccountView,
    pub member_mint_authority: AccountView,
    pub group: AccountView,
    pub group_update_authority: AccountView,
}
