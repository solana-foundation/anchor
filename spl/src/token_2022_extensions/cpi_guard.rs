// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn cpi_guard_enable(ctx: CpiContext<'_, '_, CpiGuard>) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token_2022::instructions::cpi_guard::Enable {
        account: &ctx.accounts.account,
        token_program: ctx.accounts.token_program_id.address(),
        multisig_signers: &signers,
        authority: &ctx.accounts.authority,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn cpi_guard_disable(ctx: CpiContext<'_, '_, CpiGuard>) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token_2022::instructions::cpi_guard::Disable {
        account: &ctx.accounts.account,
        token_program: ctx.accounts.token_program_id.address(),
        multisig_signers: &signers,
        authority: &ctx.accounts.authority,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct CpiGuard {
    pub token_program_id: AccountInfo,
    pub account: AccountInfo,
    pub authority: AccountInfo,
}
