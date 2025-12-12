use anchor_lang::prelude::UncheckedAccount;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Result, ToAccountInfo};

pub use ::spl_associated_token_account_interface as spl_associated_token_account;
pub use ::spl_associated_token_account_interface::{
    address::{get_associated_token_address, get_associated_token_address_with_program_id},
    program::ID,
};

pub fn create<'info>(ctx: CpiContext<'_, '_, '_, 'info, Create<'info>>) -> Result<()> {
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        ctx.accounts.payer.key,
        ctx.accounts.authority.key,
        ctx.accounts.mint.key,
        ctx.accounts.token_program.key,
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.associated_token.to_account_info(),
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn create_idempotent<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CreateIdempotent<'info>>,
) -> Result<()> {
    let ix = spl_associated_token_account::instruction::create_associated_token_account_idempotent(
        ctx.accounts.payer.key,
        ctx.accounts.authority.key,
        ctx.accounts.mint.key,
        ctx.accounts.token_program.key,
    );
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.associated_token.to_account_info(),
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct Create<'info> {
    pub payer: UncheckedAccount<'info>,
    pub associated_token: UncheckedAccount<'info>,
    pub authority: UncheckedAccount<'info>,
    pub mint: UncheckedAccount<'info>,
    pub system_program: UncheckedAccount<'info>,
    pub token_program: UncheckedAccount<'info>,
}

type CreateIdempotent<'info> = Create<'info>;

#[derive(Clone)]
pub struct AssociatedToken;

impl anchor_lang::Id for AssociatedToken {
    fn id() -> Pubkey {
        ID
    }
}
