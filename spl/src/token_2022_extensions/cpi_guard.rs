// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};
use pinocchio_token_2022::instructions::extensions::;

pub fn cpi_guard_enable(ctx: CpiContext<'_, '_, CpiGuard>) -> Result<()> {
    let ix = CpiGuardEnable {
        token_program_id: &ctx.accounts.token_program_id,
        account: &ctx.accounts.account,
        owner: &ctx.accounts.owner,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn cpi_guard_disable(ctx: CpiContext<'_, '_, CpiGuard>) -> Result<()> {
    let ix = todo!();

    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct CpiGuard {
    pub token_program_id: AccountInfo,
    pub account: AccountInfo,
    pub owner: AccountInfo,
}
