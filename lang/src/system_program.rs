use crate::prelude::*;
use arch_program::pubkey::Pubkey;

pub use arch_program::system_program::SYSTEM_PROGRAM_ID;

#[derive(Debug, Clone)]
pub struct System;

impl satellite_lang::Id for System {
    fn id() -> Pubkey {
        SYSTEM_PROGRAM_ID
    }
}

// ---------------------------------------------------------------------
// Allocate
// ---------------------------------------------------------------------
#[derive(Accounts)]
pub struct Allocate<'info> {
    pub account_to_allocate: AccountInfo<'info>,
}

pub fn allocate<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, Allocate<'info>>,
    space: u64,
) -> Result<()> {
    let ix = crate::arch_program::system_instruction::allocate(
        ctx.accounts.account_to_allocate.key,
        space,
    );
    crate::arch_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.account_to_allocate],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

// ---------------------------------------------------------------------
// Assign
// ---------------------------------------------------------------------
#[derive(Accounts)]
pub struct Assign<'info> {
    pub account_to_assign: AccountInfo<'info>,
}

pub fn assign<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, Assign<'info>>,
    owner: &Pubkey,
) -> Result<()> {
    let ix =
        crate::arch_program::system_instruction::assign(ctx.accounts.account_to_assign.key, owner);
    crate::arch_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.account_to_assign],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

// ---------------------------------------------------------------------
// Create Account
// ---------------------------------------------------------------------
#[derive(Accounts)]
pub struct CreateAccount<'info> {
    pub from: AccountInfo<'info>,
    pub to: AccountInfo<'info>,
}

pub fn create_account<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CreateAccount<'info>>,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Result<()> {
    let ix = crate::arch_program::system_instruction::create_account(
        ctx.accounts.from.key,
        ctx.accounts.to.key,
        lamports,
        space,
        owner,
    );
    crate::arch_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.from, ctx.accounts.to],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

// ---------------------------------------------------------------------
// Transfer
// ---------------------------------------------------------------------
#[derive(Accounts)]
pub struct Transfer<'info> {
    pub from: AccountInfo<'info>,
    pub to: AccountInfo<'info>,
}

pub fn transfer<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, Transfer<'info>>,
    lamports: u64,
) -> Result<()> {
    let ix = crate::arch_program::system_instruction::transfer(
        ctx.accounts.from.key,
        ctx.accounts.to.key,
        lamports,
    );
    crate::arch_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.from, ctx.accounts.to],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}
