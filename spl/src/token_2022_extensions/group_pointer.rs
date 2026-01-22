// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn group_pointer_initialize(
    ctx: CpiContext<'_, '_, GroupPointerInitialize>,
    authority: Option<Pubkey>,
    group_address: Option<Pubkey>,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupPointerInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn group_pointer_update(
    ctx: CpiContext<'_, '_, GroupPointerUpdate>,
    group_address: Option<Pubkey>,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupPointerUpdate {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
}
