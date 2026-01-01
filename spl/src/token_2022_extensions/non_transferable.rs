// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn non_transferable_mint_initialize(
    ctx: CpiContext<'_, '_, NonTransferableMintInitialize>,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct NonTransferableMintInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}
