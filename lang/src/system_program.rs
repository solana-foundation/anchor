// Avoiding AccountView deprecated msg in anchor context
#![allow(deprecated)]
pub use crate::pinocchio_runtime::system_program::ID;
use crate::{pinocchio_runtime::pubkey::Pubkey, prelude::*};

#[derive(Debug, Clone)]
pub struct System;

impl anchor_lang::Id for System {
    fn id() -> Pubkey {
        ID
    }
}

#[derive(Accounts)]
pub struct AdvanceNonceAccount {
    pub nonce: AccountView,
    pub authorized: AccountView,
    pub recent_blockhashes: AccountView,
}
pub fn advance_nonce_account(ctx: CpiContext<'_, '_, AdvanceNonceAccount>) -> Result<()> {
    let instruction = system_instruction::AdvanceNonceAccount {
        account: &ctx.accounts.nonce,
        recent_blockhashes_sysvar: &ctx.accounts.recent_blockhashes,
        authority: &ctx.accounts.authorized,
    };

    instruction
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct Allocate {
    pub account_to_allocate: AccountView,
}

pub fn allocate(ctx: CpiContext<'_, '_, Allocate>, space: u64) -> Result<()> {
    let instruction = system_instruction::Allocate {
        account: &ctx.accounts.account_to_allocate,
        space,
    };
    instruction
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct AllocateWithSeed {
    pub account_to_allocate: AccountView,
    pub base: AccountView,
}

pub fn allocate_with_seed(
    ctx: CpiContext<'_, '_, AllocateWithSeed>,
    seed: &str,
    space: u64,
    owner: &Pubkey,
) -> Result<()> {
    let instruction = system_instruction::AllocateWithSeed {
        account: &ctx.accounts.account_to_allocate,
        base: &ctx.accounts.base,
        seed,
        space,
        owner,
    };
    instruction
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct Assign {
    pub account_to_assign: AccountView,
}

pub fn assign(ctx: CpiContext<'_, '_, Assign>, owner: &Pubkey) -> Result<()> {
    // Build instruction accounts
    let instruction = system_instruction::Assign {
        account: &ctx.accounts.account_to_assign,
        owner,
    };

    instruction
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct AssignWithSeed {
    pub account_to_assign: AccountView,
    pub base: AccountView,
}

pub fn assign_with_seed(
    ctx: CpiContext<'_, '_, AssignWithSeed>,
    seed: &str,
    owner: &Pubkey,
) -> Result<()> {
    let instruction = system_instruction::AssignWithSeed {
        account: &ctx.accounts.account_to_assign,
        base: &ctx.accounts.base,
        seed,
        owner,
    };
    instruction
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct AuthorizeNonceAccount {
    pub nonce: AccountView,
    pub authorized: AccountView,
}

pub fn authorize_nonce_account(
    ctx: CpiContext<'_, '_, AuthorizeNonceAccount>,
    new_authority: &Pubkey,
) -> Result<()> {
    let instruction = system_instruction::AuthorizeNonceAccount {
        new_authority,
        account: &ctx.accounts.nonce,
        authority: &ctx.accounts.authorized,
    };

    instruction
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct CreateAccount {
    pub from: AccountView,
    pub to: AccountView,
}

pub fn create_account(
    ctx: CpiContext<'_, '_, CreateAccount>,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Result<()> {
    let instruction = system_instruction::CreateAccount {
        from: &ctx.accounts.from,
        to: &ctx.accounts.to,
        lamports,
        space,
        owner,
    };
    instruction
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct CreateAccountWithSeed {
    pub from: AccountView,
    pub to: AccountView,
    pub base: AccountView,
}

pub fn create_account_with_seed(
    ctx: CpiContext<'_, '_, CreateAccountWithSeed>,
    seed: &str,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Result<()> {
    let instruction = system_instruction::CreateAccountWithSeed {
        from: &ctx.accounts.from,
        to: &ctx.accounts.to,
        base: Some(&ctx.accounts.base),
        seed,
        lamports,
        space,
        owner,
    };
    instruction
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct CreateNonceAccount {
    pub from: AccountView,
    pub nonce: AccountView,
    pub recent_blockhashes: AccountView,
    pub rent: AccountView,
}

pub fn create_nonce_account(
    ctx: CpiContext<'_, '_, CreateNonceAccount>,
    lamports: u64,
    authority: &Pubkey,
) -> Result<()> {
    const NONCE_STATE_SIZE: u64 = 80;
    let create_ix = system_instruction::CreateAccount {
        from: &ctx.accounts.from,
        to: &ctx.accounts.nonce,
        lamports,
        space: NONCE_STATE_SIZE,
        owner: &ID,
    };
    create_ix
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)?;
    let init_ix = system_instruction::InitializeNonceAccount {
        account: &ctx.accounts.nonce,
        recent_blockhashes_sysvar: &ctx.accounts.recent_blockhashes,
        rent_sysvar: &ctx.accounts.rent,
        authority,
    };
    init_ix.invoke().map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct CreateNonceAccountWithSeed {
    pub from: AccountView,
    pub nonce: AccountView,
    pub base: AccountView,
    pub recent_blockhashes: AccountView,
    pub rent: AccountView,
}

pub fn create_nonce_account_with_seed(
    ctx: CpiContext<'_, '_, CreateNonceAccountWithSeed>,
    lamports: u64,
    seed: &str,
    authority: &Pubkey,
) -> Result<()> {
    const NONCE_STATE_SIZE: u64 = 80;
    let create_ix = system_instruction::CreateAccountWithSeed {
        from: &ctx.accounts.from,
        to: &ctx.accounts.nonce,
        base: Some(&ctx.accounts.base),
        seed,
        lamports,
        space: NONCE_STATE_SIZE,
        owner: &ID,
    };
    create_ix
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)?;
    let init_ix = system_instruction::InitializeNonceAccount {
        account: &ctx.accounts.nonce,
        recent_blockhashes_sysvar: &ctx.accounts.recent_blockhashes,
        rent_sysvar: &ctx.accounts.rent,
        authority,
    };
    init_ix.invoke().map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct InitializeNonceAccount {
    pub nonce: AccountView,
    pub base: AccountView,
    pub recent_blockhashes: AccountView,
    pub rent: AccountView,
}

pub fn initialize_nonce_account(
    ctx: CpiContext<'_, '_, InitializeNonceAccount>,
    authority: &Pubkey,
) -> Result<()> {
    let instruction = system_instruction::InitializeNonceAccount {
        account: &ctx.accounts.nonce,
        recent_blockhashes_sysvar: &ctx.accounts.recent_blockhashes,
        rent_sysvar: &ctx.accounts.rent,
        authority,
    };
    instruction.invoke().map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct Transfer {
    pub from: AccountView,
    pub to: AccountView,
}

pub fn transfer(ctx: CpiContext<'_, '_, Transfer>, lamports: u64) -> Result<()> {
    let instruction = system_instruction::Transfer {
        from: &ctx.accounts.from,
        to: &ctx.accounts.to,
        lamports,
    };
    instruction
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct TransferWithSeed {
    pub from: AccountView,
    pub base: AccountView,
    pub to: AccountView,
}

pub fn transfer_with_seed(
    ctx: CpiContext<'_, '_, TransferWithSeed>,
    seed: &str,
    owner: &Pubkey,
    lamports: u64,
) -> Result<()> {
    let instruction = system_instruction::TransferWithSeed {
        from: &ctx.accounts.from,
        base: &ctx.accounts.base,
        to: &ctx.accounts.to,
        seed,
        lamports,
        owner,
    };

    instruction
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct WithdrawNonceAccount {
    pub nonce: AccountView,
    pub to: AccountView,
    pub recent_blockhashes: AccountView,
    pub rent: AccountView,
    pub authorized: AccountView,
}

pub fn withdraw_nonce_account(
    ctx: CpiContext<'_, '_, WithdrawNonceAccount>,
    lamports: u64,
) -> Result<()> {
    let instruction = system_instruction::WithdrawNonceAccount {
        account: &ctx.accounts.nonce,
        recipient: &ctx.accounts.to,
        recent_blockhashes_sysvar: &ctx.accounts.recent_blockhashes,
        rent_sysvar: &ctx.accounts.rent,
        authority: &ctx.accounts.authorized,
        lamports,
    };

    instruction
        .invoke_signed(ctx.signer_seeds)
        .map_err(error::Error::from)
}
