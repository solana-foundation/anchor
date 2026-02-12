// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::{context::CpiContext, Accounts};
use anchor_lang::{Key, Result};
use pinocchio_token_2022::state::AccountState;

pub fn default_account_state_initialize(
    ctx: CpiContext<'_, '_, DefaultAccountStateInitialize>,
    state: &AccountState,
) -> Result<()> {
    let ix = pinocchio_token_2022::instructions::default_account_state::initialize::Initialize {
        mint: &ctx.accounts.mint,
        state: state.clone() as u8,
        token_program: &ctx.accounts.token_program_id.address(),
    };
    ix.invoke().map_err(Into::into)
}

#[derive(Accounts)]
pub struct DefaultAccountStateInitialize {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
}

pub fn default_account_state_update(
    ctx: CpiContext<'_, '_, DefaultAccountStateUpdate>,
    state: &AccountState,
) -> Result<()> {
    let signers: Vec<&AccountInfo> = ctx.remaining_accounts.iter().collect();

    let ix = pinocchio_token_2022::instructions::default_account_state::update::Update {
        mint: &ctx.accounts.mint,
        freeze_authority: &ctx.accounts.freeze_authority,
        signers: &signers,
        state: state.clone() as u8,
        token_program: &ctx.accounts.token_program_id.address(),
    };
    ix.invoke_signed(ctx.signer_seeds).map_err(Into::into)
}

#[derive(Accounts)]
pub struct DefaultAccountStateUpdate {
    pub token_program_id: AccountInfo,
    pub mint: AccountInfo,
    pub freeze_authority: AccountInfo,
}
