// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::Result;
use anchor_lang::{context::CpiContext, Accounts};

pub fn token_group_initialize(
    ctx: CpiContext<'_, '_, 'static, TokenGroupInitialize>,
    update_authority: Option<Pubkey>,
    max_size: u64,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(
        ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenGroupInitialize {
    pub program_id: AccountInfo,
    pub group: AccountInfo,
    pub mint: AccountInfo,
    pub mint_authority: AccountInfo,
}

pub fn token_member_initialize(
    ctx: CpiContext<'_, '_, 'static, TokenMemberInitialize>,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct TokenMemberInitialize {
    pub program_id: AccountInfo,
    pub member: AccountInfo,
    pub member_mint: AccountInfo,
    pub member_mint_authority: AccountInfo,
    pub group: AccountInfo,
    pub group_update_authority: AccountInfo,
}
