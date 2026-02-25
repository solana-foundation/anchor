// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn permanent_delegate_initialize(
    ctx: CpiContext<'_, '_, PermanentDelegateInitialize>,
    permanent_delegate: &Pubkey,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::permanent_delegate::InitializePermanentDelegate {
        token_program: ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        delegate: permanent_delegate,
    };
    ix.invoke().map_err(Into::into)
}

#[derive(Accounts)]
pub struct PermanentDelegateInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}
