use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::Result;
use anchor_lang::{context::CpiContext, Accounts};
use spl_token_2022::state::AccountState;

pub fn default_account_state_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, DefaultAccountStateInitialize<'info>>,
    state: &AccountState,
) -> Result<()> {
    let ix = spl_token_2022::extension::default_account_state::instruction::initialize_default_account_state(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        state
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct DefaultAccountStateInitialize<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}

pub fn default_account_state_update<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, DefaultAccountStateUpdate<'info>>,
    state: &AccountState,
) -> Result<()> {
    let ix = spl_token_2022::extension::default_account_state::instruction::update_default_account_state(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        ctx.accounts.freeze_authority.key,
        &[],
        state
    )?;

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.mint,
            ctx.accounts.freeze_authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct DefaultAccountStateUpdate<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub freeze_authority: AccountInfo<'info>,
}
