// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::Result;
use anchor_lang::{context::CpiContext, Accounts};
use spl_token_2022_interface as spl_token_2022;

pub fn cpi_guard_enable(ctx: CpiContext<'_, '_, 'static, CpiGuard>) -> Result<()> {
    let ix = spl_token_2022::extension::cpi_guard::instruction::enable_cpi_guard(
        ctx.accounts.token_program_id.key,
        ctx.accounts.account.key,
        ctx.accounts.account.owner,
        &[],
    )?;
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)

}

pub fn cpi_guard_disable(ctx: CpiContext<'_, '_, 'static, CpiGuard>) -> Result<()> {
    let ix = spl_token_2022::extension::cpi_guard::instruction::disable_cpi_guard(
        ctx.accounts.token_program_id.key,
        ctx.accounts.account.key,
        ctx.accounts.account.owner,
        &[],
    )?;

    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct CpiGuard {
    pub token_program_id: AccountInfo,
    pub account: AccountInfo,
    pub owner: AccountInfo,
}
