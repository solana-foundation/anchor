// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn transfer_fee_initialize(
    ctx: CpiContext<'_, '_, TransferFeeInitialize>,
    transfer_fee_config_authority: Option<&Pubkey>,
    withdraw_withheld_authority: Option<&Pubkey>,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::transfer_fee::InitializeTransferFeeConfig {
        token_program: ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        transfer_fee_config_authority,
        withdraw_withheld_authority,
        transfer_fee_basis_points,
        maximum_fee,
    };
    ix.invoke().map_err(Into::into)
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
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token_2022::instructions::transfer_fee::SetTransferFee {
        token_program: ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        authority: &ctx.accounts.authority,
        transfer_fee_basis_points,
        maximum_fee,
        multisig_signers: &signers,
    };
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
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();
    let ix = pinocchio_token_2022::instructions::transfer_fee::TransferCheckedWithFee {
        token_program: ctx.accounts.token_program_id.address(),
        source: &ctx.accounts.source,
        mint: &ctx.accounts.mint,
        destination: &ctx.accounts.destination,
        authority: &ctx.accounts.authority,
        amount,
        decimals,
        fee,
        multisig_signers: &signers,
    };
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
    sources: Vec<&AccountInfo>,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::transfer_fee::HarvestWithheldTokensToMint {
        token_program: ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        sources: &sources,
    };
    ix.invoke().map_err(Into::into)
}

#[derive(Accounts)]
pub struct HarvestWithheldTokensToMint {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn withdraw_withheld_tokens_from_mint(
    ctx: CpiContext<'_, '_, WithdrawWithheldTokensFromMint>,
) -> Result<()> {
    let signers = ctx.remaining_accounts.iter().collect::<Vec<&AccountInfo>>();
    let ix = pinocchio_token_2022::instructions::transfer_fee::WithdrawWithheldTokensFromMint {
        token_program: ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        destination: &ctx.accounts.destination,
        authority: &ctx.accounts.authority,
        multisig_signers: &signers,
    };
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
    sources: Vec<&AccountInfo>,
) -> Result<()> {
    let signers = ctx.remaining_accounts.iter().collect::<Vec<&AccountInfo>>();
    let ix = pinocchio_token_2022::instructions::transfer_fee::WithdrawWithheldTokensFromAccounts {
        token_program: ctx.accounts.token_program_id.address(),
        mint: &ctx.accounts.mint,
        sources: &sources,
        destination: &ctx.accounts.destination,
        authority: &ctx.accounts.authority,
        multisig_signers: &signers,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct WithdrawWithheldTokensFromAccounts {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub destination: AccountInfo,
    pub authority: AccountInfo,
}
