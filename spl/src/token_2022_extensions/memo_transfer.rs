// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn memo_transfer_enable(ctx: CpiContext<'_, '_, MemoTransfer>) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token_2022::instructions::memo_transfer::Enable {
        token_account: &ctx.accounts.account,
        authority: &ctx.accounts.authority,
        signers: &signers,
        token_program: &ctx.accounts.token_program_id.address(),
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn memo_transfer_disable(ctx: CpiContext<'_, '_, MemoTransfer>) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token_2022::instructions::memo_transfer::Disable {
        token_account: &ctx.accounts.account,
        authority: &ctx.accounts.authority,
        signers: &signers,
        token_program: &ctx.accounts.token_program_id.address(),
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct MemoTransfer {
    pub token_program_id: AccountInfo,
    pub account: AccountInfo,
    pub authority: AccountInfo,
}
