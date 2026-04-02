// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use {
    anchor_lang::{
        context::CpiContext,
        pinocchio_runtime::{account_view::AccountView, pubkey::Pubkey},
        Accounts, Result,
    },
    spl_token_2022::state::AccountState,
    spl_token_2022_interface as spl_token_2022,
};

pub fn default_account_state_initialize(
    ctx: CpiContext<'_, '_, DefaultAccountStateInitialize>,
    state: &AccountState,
) -> Result<()> {
    let ix = spl_token_2022::extension::default_account_state::instruction::initialize_default_account_state(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        state
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct DefaultAccountStateInitialize {
    pub token_program_id: AccountView,
    pub mint: AccountView,
}

pub fn default_account_state_update(
    ctx: CpiContext<'_, '_, DefaultAccountStateUpdate>,
    state: &AccountState,
) -> Result<()> {
    let ix = spl_token_2022::extension::default_account_state::instruction::update_default_account_state(
        ctx.accounts.token_program_id.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.freeze_authority.address(),
        &[],
        state
    )?;

    anchor_lang::pinocchio_runtime::program::invoke_signed(
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
pub struct DefaultAccountStateUpdate {
    pub token_program_id: AccountView,
    pub mint: AccountView,
    pub freeze_authority: AccountView,
}
