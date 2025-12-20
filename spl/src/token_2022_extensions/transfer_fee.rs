// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{Result, Key};
use anchor_lang::{context::CpiContext, Accounts};

pub fn transfer_fee_initialize(
    ctx: CpiContext<'_, '_, TransferFeeInitialize>,
    transfer_fee_config_authority: Option<&Pubkey>,
    withdraw_withheld_authority: Option<&Pubkey>,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct TransferFeeInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn transfer_fee_set(
    ctx: CpiContext<'_, '_, TransferFeeSetTransferFee>,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct TransferFeeSetTransferFee {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
}

pub fn transfer_checked_with_fee(
    ctx: CpiContext<'_, '_, TransferCheckedWithFee>,
    amount: u64,
    decimals: u8,
    fee: u64,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct TransferCheckedWithFee {
    pub token_program_id: AccountInfo,
    pub source: AccountInfo,
    pub mint: AccountInfo,
    pub destination: AccountInfo,
    pub authority: AccountInfo,
}

pub fn harvest_withheld_tokens_to_mint(
    ctx: CpiContext<'_, '_, HarvestWithheldTokensToMint>,
    sources: Vec<AccountInfo>,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct HarvestWithheldTokensToMint {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn withdraw_withheld_tokens_from_mint(
    ctx: CpiContext<'_, '_, WithdrawWithheldTokensFromMint>,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct WithdrawWithheldTokensFromMint {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub destination: AccountInfo,
    pub authority: AccountInfo,
}

pub fn withdraw_withheld_tokens_from_accounts(
    ctx: CpiContext<'_, '_, WithdrawWithheldTokensFromAccounts>,
    sources: Vec<AccountInfo>,
) -> Result<()> {
    let ix = todo!();
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct WithdrawWithheldTokensFromAccounts {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub destination: AccountInfo,
    pub authority: AccountInfo,
}
