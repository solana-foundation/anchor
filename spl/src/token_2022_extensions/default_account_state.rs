// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::Result;
use anchor_lang::{context::CpiContext, Accounts};
use pinocchio_token_2022::state::AccountState;

pub fn default_account_state_initialize(
    ctx: CpiContext<'_, '_, 'static, DefaultAccountStateInitialize>,
    state: &AccountState,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct DefaultAccountStateInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn default_account_state_update(
    ctx: CpiContext<'_, '_, 'static, DefaultAccountStateUpdate>,
    state: &AccountState,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct DefaultAccountStateUpdate {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub freeze_authority: AccountInfo,
}
