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

pub fn non_transferable_mint_initialize(
    ctx: CpiContext<'_, '_, NonTransferableMintInitialize>,
) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_non_transferable_mint(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
    )?;
    crate::cpi_util::invoke_signed_solana_instruction(ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct NonTransferableMintInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}
