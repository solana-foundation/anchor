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

pub fn cpi_guard_enable(ctx: CpiContext<'_, '_, CpiGuard>) -> Result<()> {
    let ix = spl_token_2022::extension::cpi_guard::instruction::enable_cpi_guard(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.account.address(),
        ctx.accounts.account.owner(),
        &[],
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.account,
            ctx.accounts.owner,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn cpi_guard_disable(ctx: CpiContext<'_, '_, CpiGuard>) -> Result<()> {
    let ix = spl_token_2022::extension::cpi_guard::instruction::disable_cpi_guard(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.account.address(),
        ctx.accounts.account.owner(),
        &[],
    )?;

    crate::cpi_util::invoke_signed_solana_instruction(
        ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.account,
            ctx.accounts.owner,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct CpiGuard {
    pub token_program_id: AccountInfo,
    pub account: AccountInfo,
    pub owner: AccountInfo,
}
