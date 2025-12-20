// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{Result, Key};
use anchor_lang::{context::CpiContext, Accounts};

pub fn permanent_delegate_initialize(
    ctx: CpiContext<'_, '_, PermanentDelegateInitialize>,
    permanent_delegate: &Pubkey,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct PermanentDelegateInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}
