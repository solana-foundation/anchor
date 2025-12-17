// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use crate::prelude::*;

use crate::pinocchio_runtime::pubkey::Pubkey;
pub use crate::pinocchio_runtime::system_program::ID;

#[derive(Debug, Clone)]
pub struct System;

impl anchor_lang::Id for System {
    fn id() -> Pubkey {
        ID
    }
}

#[derive(Accounts)]
pub struct AdvanceNonceAccount {
    pub nonce: AccountInfo,
    pub authorized: AccountInfo,
    pub recent_blockhashes: AccountInfo,
}
pub fn advance_nonce_account(ctx: CpiContext<'_, '_, 'static, AdvanceNonceAccount>) -> Result<()> {
    let instruction = system_instruction::AdvanceNonceAccount {
        account: &ctx.accounts.nonce,
        recent_blockhashes_sysvar: &ctx.accounts.recent_blockhashes,
        authority: &ctx.accounts.authorized,
    };

    instruction
        .invoke_signed(&ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct Allocate {
    pub account_to_allocate: AccountInfo,
}

pub fn allocate(ctx: CpiContext<'_, '_, 'static, Allocate>, space: u64) -> Result<()> {
    let instruction = system_instruction::Allocate {
        account: &ctx.accounts.account_to_allocate,
        space: space,
    };
    instruction
        .invoke_signed(&ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct AllocateWithSeed {
    pub account_to_allocate: AccountInfo,
    pub base: AccountInfo,
}

pub fn allocate_with_seed(
    ctx: CpiContext<'_, '_, 'static, AllocateWithSeed>,
    seed: &str,
    space: u64,
    owner: &Pubkey,
) -> Result<()> {
    let instruction = system_instruction::AllocateWithSeed {
        account: &ctx.accounts.account_to_allocate,
        base: &ctx.accounts.base,
        seed: seed,
        space: space,
        owner: owner,
    };
    instruction
        .invoke_signed(&ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct Assign {
    pub account_to_assign: AccountInfo,
}

pub fn assign(ctx: CpiContext<'_, '_, 'static, Assign>, owner: &Pubkey) -> Result<()> {
    // Build instruction accounts
    let instruction = system_instruction::Assign {
        account: &ctx.accounts.account_to_assign,
        owner: owner,
    };

    instruction
        .invoke_signed(&ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct AssignWithSeed {
    pub account_to_assign: AccountInfo,
    pub base: AccountInfo,
}

pub fn assign_with_seed(
    ctx: CpiContext<'_, '_, 'static, AssignWithSeed>,
    seed: &str,
    owner: &Pubkey,
) -> Result<()> {
    let instruction = system_instruction::AssignWithSeed {
        account: &ctx.accounts.account_to_assign,
        base: &ctx.accounts.base,
        seed: seed,
        owner: owner,
    };
    instruction
        .invoke_signed(&ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct AuthorizeNonceAccount {
    pub nonce: AccountInfo,
    pub authorized: AccountInfo,
}

pub fn authorize_nonce_account(
    ctx: CpiContext<'_, '_, 'static, AuthorizeNonceAccount>,
    new_authority: &Pubkey,
) -> Result<()> {
    let instruction = system_instruction::AuthorizeNonceAccount {
        new_authority: new_authority,
        account: &ctx.accounts.nonce,
        authority: &ctx.accounts.authorized,
    };

    instruction
        .invoke_signed(&ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct CreateAccount {
    pub from: AccountInfo,
    pub to: AccountInfo,
}

pub fn create_account(
    ctx: CpiContext<'_, '_, 'static, CreateAccount>,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Result<()> {
    let instruction = system_instruction::CreateAccount {
        from: &ctx.accounts.from,
        to: &ctx.accounts.to,
        lamports: lamports,
        space: space,
        owner: owner,
    };
    instruction
        .invoke_signed(&ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct CreateAccountWithSeed {
    pub from: AccountInfo,
    pub to: AccountInfo,
    pub base: AccountInfo,
}

pub fn create_account_with_seed(
    ctx: CpiContext<'_, '_, 'static, CreateAccountWithSeed>,
    seed: &str,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Result<()> {
    let instruction = system_instruction::CreateAccountWithSeed {
        from: &ctx.accounts.from,
        to: &ctx.accounts.to,
        base: Some(&ctx.accounts.base),
        seed: seed,
        lamports: lamports,
        space: space,
        owner: owner,
    };
    instruction
        .invoke_signed(&ctx.signer_seeds)
        .map_err(error::Error::from)
}

// #[derive(Accounts)]
// pub struct CreateNonceAccount {
//     pub from: AccountInfo,
//     pub nonce: AccountInfo,
//     pub recent_blockhashes: AccountInfo,
//     pub rent: AccountInfo,
// }

// pub fn create_nonce_account(
//     ctx: CpiContext<'_, '_, 'info, CreateNonceAccount>,
//     lamports: u64,
//     authority: &Pubkey,
// ) -> Result<()> {
//     let instruction = system_instruction::CreateNonceAccount {
//         nonce: &ctx.accounts.nonce,
//         recent_blockhashes: &ctx.accounts.recent_blockhashes,
//         rent: &ctx.accounts.rent,
//         authority: authority.to_bytes(),
//         authority,
//         lamports,
//     };
//     instruction
//         .invoke_signed(&ctx.signer_seeds)
//         .map_err(error::Error::from)
// }

// pub struct CreateNonceAccountWithSeed {
//     pub from: AccountInfo,
//     pub nonce: AccountInfo,
//     pub base: AccountInfo,
//     pub recent_blockhashes: AccountInfo,
//     pub rent: AccountInfo,
// }

// pub fn create_nonce_account_with_seed(
//     ctx: CpiContext<'_, '_, 'info, CreateNonceAccountWithSeed>,
//     lamports: u64,
//     seed: &str,
//     authority: &Pubkey,
// ) -> Result<()> {
//     let instruction = system_instruction::CreateNonceAccountWithSeed {
//         from: &ctx.accounts.from,
//         nonce: &ctx.accounts.nonce,
//         base: &ctx.accounts.base,
//         recent_blockhashes: &ctx.accounts.recent_blockhashes,
//         rent: &ctx.accounts.rent,
//         seed: seed.to_string(),
//         authority: authority.to_bytes(),
//     };
//     instruction
//         .invoke_signed(&ctx.signer_seeds)
//         .map_err(error::Error::from)
// }

#[derive(Accounts)]
pub struct InitializeNonceAccount {
    pub nonce: AccountInfo,
    pub base: AccountInfo,
    pub recent_blockhashes: AccountInfo,
    pub rent: AccountInfo,
}

pub fn initialize_nonce_account(
    ctx: CpiContext<'_, '_, 'static, InitializeNonceAccount>,
    authority: &Pubkey,
) -> Result<()> {
    let instruction = system_instruction::InitializeNonceAccount {
        account: &ctx.accounts.nonce,
        recent_blockhashes_sysvar: &ctx.accounts.recent_blockhashes,
        rent_sysvar: &ctx.accounts.rent,
        authority: authority,
    };
    instruction.invoke().map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct Transfer {
    pub from: AccountInfo,
    pub to: AccountInfo,
}

pub fn transfer(ctx: CpiContext<'_, '_, 'static, Transfer>, lamports: u64) -> Result<()> {
    let instruction = system_instruction::Transfer {
        from: &ctx.accounts.from,
        to: &ctx.accounts.to,
        lamports: lamports,
    };
    instruction
        .invoke_signed(&ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct TransferWithSeed {
    pub from: AccountInfo,
    pub base: AccountInfo,
    pub to: AccountInfo,
}

pub fn transfer_with_seed(
    ctx: CpiContext<'_, '_, 'static, TransferWithSeed>,
    seed: &str,
    owner: &Pubkey,
    lamports: u64,
) -> Result<()> {
    let instruction = system_instruction::TransferWithSeed {
        from: &ctx.accounts.from,
        base: &ctx.accounts.base,
        to: &ctx.accounts.to,
        seed: seed,
        lamports: lamports,
        owner: owner,
    };

    instruction
        .invoke_signed(&ctx.signer_seeds)
        .map_err(error::Error::from)
}

#[derive(Accounts)]
pub struct WithdrawNonceAccount {
    pub nonce: AccountInfo,
    pub to: AccountInfo,
    pub recent_blockhashes: AccountInfo,
    pub rent: AccountInfo,
    pub authorized: AccountInfo,
}

pub fn withdraw_nonce_account(
    ctx: CpiContext<'_, '_, 'static, WithdrawNonceAccount>,
    lamports: u64,
) -> Result<()> {
    let instruction = system_instruction::WithdrawNonceAccount {
        account: &ctx.accounts.nonce,
        recipient: &ctx.accounts.to,
        recent_blockhashes_sysvar: &ctx.accounts.recent_blockhashes,
        rent_sysvar: &ctx.accounts.rent,
        authority: &ctx.accounts.authorized,
        lamports: lamports,
    };

    instruction
        .invoke_signed(&ctx.signer_seeds)
        .map_err(error::Error::from)
}
