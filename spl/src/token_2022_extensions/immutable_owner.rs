use anchor_lang::prelude::UncheckedAccount;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Result, ToAccountInfo};
use spl_token_2022_interface as spl_token_2022;

pub fn immutable_owner_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, ImmutableOwnerInitialize<'info>>,
) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_immutable_owner(
        ctx.accounts.token_program_id.key,
        ctx.accounts.token_account.key,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id.to_account_info(),
            ctx.accounts.token_account.to_account_info(),
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct ImmutableOwnerInitialize<'info> {
    pub token_program_id: UncheckedAccount<'info>,
    pub token_account: UncheckedAccount<'info>,
}
