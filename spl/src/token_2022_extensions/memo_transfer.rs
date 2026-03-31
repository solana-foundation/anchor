// Avoiding AccountView deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::{
    context::CpiContext,
    pinocchio_runtime::{account_view::AccountView, pubkey::Pubkey},
    Accounts, Key, Result,
};

pub fn memo_transfer_enable(ctx: CpiContext<'_, '_, MemoTransfer>) -> Result<()> {
    let signers: Vec<&AccountView> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token_2022::instructions::memo_transfer::Enable {
        account: &ctx.accounts.account,
        authority: &ctx.accounts.authority,
        multisig_signers: &signers,
        token_program: ctx.accounts.token_program_id.address(),
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn memo_transfer_disable(ctx: CpiContext<'_, '_, MemoTransfer>) -> Result<()> {
    let signers: Vec<&AccountView> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token_2022::instructions::memo_transfer::Disable {
        account: &ctx.accounts.account,
        authority: &ctx.accounts.authority,
        multisig_signers: &signers,
        token_program: ctx.accounts.token_program_id.address(),
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct MemoTransfer {
    pub token_program_id: AccountView,
    pub account: AccountView,
    pub authority: AccountView,
}
