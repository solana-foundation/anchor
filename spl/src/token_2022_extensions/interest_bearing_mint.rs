// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn interest_bearing_mint_initialize(
    ctx: CpiContext<'_, '_, InterestBearingMintInitialize>,
    rate_authority: Option<Pubkey>,
    rate: i16,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct InterestBearingMintInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn interest_bearing_mint_update_rate(
    ctx: CpiContext<'_, '_, InterestBearingMintUpdateRate>,
    rate: i16,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct InterestBearingMintUpdateRate {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub rate_authority: AccountInfo,
}
