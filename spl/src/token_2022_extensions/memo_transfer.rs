// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::Result;
use anchor_lang::{context::CpiContext, Accounts};

pub fn memo_transfer_initialize(ctx: CpiContext<'_, '_, 'static, MemoTransfer>) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn memo_transfer_disable(ctx: CpiContext<'_, '_, 'static, MemoTransfer>) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct MemoTransfer {
    pub token_program_id: AccountInfo,
    pub account: AccountInfo,
    pub owner: AccountInfo,
}
