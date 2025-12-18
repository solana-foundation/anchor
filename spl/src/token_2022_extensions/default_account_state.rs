use anchor_lang::context::CpiContext;
use anchor_lang::prelude::AccountInfo;
use anchor_lang::{Result, ToAccountInfos};
use spl_token_2022::state::AccountState;
use spl_token_2022_interface as spl_token_2022;

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

pub struct DefaultAccountStateInitialize<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for DefaultAccountStateInitialize<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.token_program_id.to_owned(), self.mint.to_owned()]
    }
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

pub struct DefaultAccountStateUpdate<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub freeze_authority: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for DefaultAccountStateUpdate<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.token_program_id.to_owned(),
            self.mint.to_owned(),
            self.freeze_authority.to_owned(),
        ]
    }
}
