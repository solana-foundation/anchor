// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
pub use ::spl_associated_token_account_interface::{
    self as spl_associated_token_account,
    address::{get_associated_token_address, get_associated_token_address_with_program_id},
    program::ID,
};
use anchor_lang::{
    context::CpiContext,
    pinocchio_runtime::{account_view::AccountView, pubkey::Pubkey},
    Accounts, Key, Result,
};

pub fn create(ctx: CpiContext<'_, '_, Create>) -> Result<()> {
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        ctx.accounts.payer.address(),
        ctx.accounts.authority.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.token_program.address(),
    );
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.payer,
            ctx.accounts.associated_token,
            ctx.accounts.authority,
            ctx.accounts.mint,
            ctx.accounts.system_program,
            ctx.accounts.token_program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn create_idempotent(ctx: CpiContext<'_, '_, CreateIdempotent>) -> Result<()> {
    let ix = spl_associated_token_account::instruction::create_associated_token_account_idempotent(
        ctx.accounts.payer.address(),
        ctx.accounts.authority.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.token_program.address(),
    );
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.payer,
            ctx.accounts.associated_token,
            ctx.accounts.authority,
            ctx.accounts.mint,
            ctx.accounts.system_program,
            ctx.accounts.token_program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
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
        ID
    }
}
