// Avoiding AccountView deprecated msg in anchor context
#![allow(deprecated)]
pub use ::pinocchio_associated_token_account as spl_associated_token_account;
use anchor_lang::{
    context::CpiContext,
    pinocchio_runtime::{account_view::AccountView, pubkey::Pubkey},
    Accounts, Key, Result,
};

pub fn create(ctx: CpiContext<'_, '_, Create>) -> Result<()> {
    let ix = spl_associated_token_account::instructions::Create {
        funding_account: &ctx.accounts.payer,
        account: &ctx.accounts.associated_token,
        wallet: &ctx.accounts.authority,
        mint: &ctx.accounts.mint,
        system_program: &ctx.accounts.system_program,
        token_program: &ctx.accounts.token_program,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn create_idempotent(ctx: CpiContext<'_, '_, CreateIdempotent>) -> Result<()> {
    let ix = spl_associated_token_account::instructions::CreateIdempotent {
        funding_account: &ctx.accounts.payer,
        account: &ctx.accounts.associated_token,
        wallet: &ctx.accounts.authority,
        mint: &ctx.accounts.mint,
        system_program: &ctx.accounts.system_program,
        token_program: &ctx.accounts.token_program,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct Create {
    pub payer: AccountView,
    pub associated_token: AccountView,
    pub authority: AccountView,
    pub mint: AccountView,
    pub system_program: AccountView,
    pub token_program: AccountView,
}

type CreateIdempotent = Create;

#[derive(Clone)]
pub struct AssociatedToken;

impl anchor_lang::Id for AssociatedToken {
    fn id() -> Pubkey {
        spl_associated_token_account::ID
    }
}
