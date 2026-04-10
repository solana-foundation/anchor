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

pub fn group_pointer_initialize(
    ctx: CpiContext<'_, '_, GroupPointerInitialize>,
    authority: Option<Pubkey>,
    group_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::group_pointer::instruction::initialize(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        authority,
        group_address,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupPointerInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn group_pointer_update(
    ctx: CpiContext<'_, '_, GroupPointerUpdate>,
    group_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::group_pointer::instruction::update(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
        &[ctx.accounts.authority.address()],
        group_address,
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupPointerUpdate {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub authority: AccountInfo,
}
