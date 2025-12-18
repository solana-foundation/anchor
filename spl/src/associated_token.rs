// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::Result;
use anchor_lang::{context::CpiContext, Accounts};

pub use ::pinocchio_associated_token_account as spl_associated_token_account;

pub fn create(ctx: CpiContext<'_, '_, 'static, Create>) -> Result<()> {
    let ix = spl_associated_token_account::instructions::Create{
        funding_account: &ctx.accounts.payer,
        account: &ctx.accounts.associated_token,
        wallet: &ctx.accounts.authority,
        mint: &ctx.accounts.mint,
        system_program: &ctx.accounts.system_program,
        token_program: &ctx.accounts.token_program,
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

pub fn create_idempotent(
    ctx: CpiContext<'_, '_, 'static, CreateIdempotent>,
) -> Result<()> {
    let ix = spl_associated_token_account::instructions::CreateIdempotent{
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
    pub payer: AccountInfo,
    pub associated_token: AccountInfo,
    pub authority: AccountInfo,
    pub mint: AccountInfo,
    pub system_program: AccountInfo,
    pub token_program: AccountInfo,
}

type CreateIdempotent = Create;

#[derive(Clone)]
pub struct AssociatedToken;

impl anchor_lang::Id for AssociatedToken {
    fn id() -> Pubkey {
        spl_associated_token_account::ID
    }
}
