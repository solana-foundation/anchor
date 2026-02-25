// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};

pub fn mint_close_authority_initialize(
    ctx: CpiContext<'_, '_, MintCloseAuthorityInitialize>,
    authority: Option<&Pubkey>,
) -> Result<()> {
    let ix =
        pinocchio_token_2022::instructions::mint_close_authority::InitializeMintCloseAuthority {
            token_program: ctx.accounts.token_program_id.address(),
            mint: &ctx.accounts.mint,
            close_authority: authority,
        };
    ix.invoke().map_err(Into::into)
}

#[derive(Accounts)]
pub struct MintCloseAuthorityInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}
