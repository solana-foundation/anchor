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

pub fn memo_transfer_initialize(
    ctx: CpiContext<'_, '_, MemoTransfer>,
) -> Result<()> {
    let ix = spl_token_2022::extension::memo_transfer::instruction::enable_required_transfer_memos(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.account.address(),
        ctx.accounts.owner.address(),
        &[],
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.account,
            ctx.accounts.owner,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn memo_transfer_disable(
    ctx: CpiContext<'_, '_, MemoTransfer>,
) -> Result<()> {
    let ix =
        spl_token_2022::extension::memo_transfer::instruction::disable_required_transfer_memos(
            ctx.accounts.token_program_id.address(),
            ctx.accounts.account.address(),
            ctx.accounts.owner.address(),
            &[],
        )?;
    crate::cpi_util::invoke_signed_solana_instruction(ix,
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
pub struct MemoTransfer {
    pub token_program_id: AccountInfo,
    pub account: AccountInfo,
    pub owner: AccountInfo,
}
