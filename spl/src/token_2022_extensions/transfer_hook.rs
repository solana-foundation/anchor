// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{Result, Key};
use anchor_lang::{context::CpiContext, Accounts};

pub fn transfer_hook_initialize(
    ctx: CpiContext<'_, '_, TransferHookInitialize>,
    authority: Option<Pubkey>,
    transfer_hook_program_id: Option<Pubkey>,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct TransferHookInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn transfer_hook_update(
    ctx: CpiContext<'_, '_, TransferHookUpdate>,
    transfer_hook_program_id: Option<Pubkey>,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct TransferHookUpdate {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
}
