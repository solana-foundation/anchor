// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{Result, Key};
use anchor_lang::{context::CpiContext, Accounts};

pub fn group_member_pointer_initialize(
    ctx: CpiContext<'_, '_, GroupMemberPointerInitialize>,
    authority: Option<Pubkey>,
    member_address: Option<Pubkey>,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupMemberPointerInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn group_member_pointer_update(
    ctx: CpiContext<'_, '_, GroupMemberPointerUpdate>,
    member_address: Option<Pubkey>,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupMemberPointerUpdate {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
}
