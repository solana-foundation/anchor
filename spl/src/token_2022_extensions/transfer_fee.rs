// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use {
    anchor_lang::{
        context::CpiContext,
        pinocchio_runtime::{account_info::AccountInfo, pubkey::Pubkey},
        Accounts, Result,
    },
    spl_token_2022_interface as spl_token_2022,
};

pub fn transfer_fee_initialize(
    ctx: CpiContext<'_, '_, TransferFeeInitialize>,
    transfer_fee_config_authority: Option<&Pubkey>,
    withdraw_withheld_authority: Option<&Pubkey>,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> Result<()> {
    let ix = spl_token_2022::extension::transfer_fee::instruction::initialize_transfer_fee_config(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        transfer_fee_config_authority,
        withdraw_withheld_authority,
        transfer_fee_basis_points,
        maximum_fee,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
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
    let ix = spl_token_2022::extension::transfer_fee::instruction::set_transfer_fee(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
        &[],
        transfer_fee_basis_points,
        maximum_fee,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.mint,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
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
    let ix = spl_token_2022::extension::transfer_fee::instruction::transfer_checked_with_fee(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.source.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.destination.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
        decimals,
        fee,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.source,
            ctx.accounts.mint,
            ctx.accounts.destination,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
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
    let ix = spl_token_2022::extension::transfer_fee::instruction::harvest_withheld_tokens_to_mint(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        &sources
            .iter()
            .map(|a| a.address())
            .collect::<Vec<_>>(),
    )?;

    let mut account_infos = vec![ctx.accounts.token_program_id, ctx.accounts.mint];
    account_infos.extend_from_slice(&sources);

    crate::cpi_util::invoke_signed_solana_instruction(ix, &account_infos, ctx.signer_seeds)
        .map_err(Into::into)
}

#[derive(Accounts)]
pub struct HarvestWithheldTokensToMint {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn withdraw_withheld_tokens_from_mint(
    ctx: CpiContext<'_, '_, WithdrawWithheldTokensFromMint>,
) -> Result<()> {
    let ix =
        spl_token_2022::extension::transfer_fee::instruction::withdraw_withheld_tokens_from_mint(
            ctx.accounts.token_program_id.address(),
            ctx.accounts.mint.address(),
            ctx.accounts.destination.address(),
            ctx.accounts.authority.address(),
            &[],
        )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.mint,
            ctx.accounts.destination,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
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
    let ix = spl_token_2022::extension::transfer_fee::instruction::withdraw_withheld_tokens_from_accounts(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.destination.address(),
        ctx.accounts.authority.address(),
        &[],
        &sources.iter().map(|a| a.address()).collect::<Vec<_>>(),
    )?;

    let mut account_infos = vec![
        ctx.accounts.token_program_id,
        ctx.accounts.mint,
        ctx.accounts.destination,
        ctx.accounts.authority,
    ];
    account_infos.extend_from_slice(&sources);

    crate::cpi_util::invoke_signed_solana_instruction(ix, &account_infos, ctx.signer_seeds)
        .map_err(Into::into)
}

#[derive(Accounts)]
pub struct WithdrawWithheldTokensFromAccounts {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub destination: AccountInfo,
    pub authority: AccountInfo,
}
