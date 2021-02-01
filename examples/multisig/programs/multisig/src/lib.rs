//! An example of a multisig to execute arbitrary Solana transactions.

#![feature(proc_macro_hygiene)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_lang::solana_program::instruction::Instruction;
use std::convert::Into;

#[program]
pub mod multisig {
    use super::*;

    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        owners: Vec<Pubkey>,
        threshold: u64,
        nonce: u8,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        multisig.owners = owners;
        multisig.threshold = threshold;
        multisig.nonce = nonce;
        Ok(())
    }

    pub fn create_transaction(
        ctx: Context<CreateTransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
    ) -> Result<()> {
        let owner_index = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a == ctx.accounts.proposer.key)
            .ok_or(ErrorCode::InvalidOwner)?;

        let mut signers = Vec::new();
        signers.resize(ctx.accounts.multisig.owners.len(), false);
        signers[owner_index] = true;

        let tx = &mut ctx.accounts.transaction;
        tx.program_id = pid;
        tx.accounts = accs;
        tx.data = data;
        tx.signers = signers;
        tx.multisig = *ctx.accounts.multisig.to_account_info().key;
        tx.did_execute = false;

        Ok(())
    }

    pub fn approve(ctx: Context<Approve>) -> Result<()> {
        let owner_index = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a == ctx.accounts.owner.key)
            .ok_or(ErrorCode::InvalidOwner)?;

        ctx.accounts.transaction.signers[owner_index] = true;

        Ok(())
    }

    // Sets the owners field on the multisig. The only way this can be invoked
    // is via a recursive call from execute_transaction -> set_owners.
    pub fn set_owners(ctx: Context<Auth>, owners: Vec<Pubkey>) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;

        if (owners.len() as u64) < multisig.threshold {
            multisig.threshold = owners.len() as u64;
        }

        multisig.owners = owners;
        Ok(())
    }

    pub fn change_threshold(ctx: Context<Auth>, threshold: u64) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        multisig.threshold = threshold;
        Ok(())
    }

    pub fn execute_transaction(ctx: Context<ExecuteTransaction>) -> Result<()> {
        // Check we have enough signers.
        let sig_count = ctx
            .accounts
            .transaction
            .signers
            .iter()
            .filter_map(|s| match s {
                false => None,
                true => Some(true),
            })
            .collect::<Vec<_>>()
            .len() as u64;
        if sig_count < ctx.accounts.multisig.threshold {
            return Err(ErrorCode::NotEnoughSigners.into());
        }

        // Execute the multisig transaction.
        let ix: Instruction = (&*ctx.accounts.transaction).into();
        let seeds = &[
            ctx.accounts.multisig.to_account_info().key.as_ref(),
            &[ctx.accounts.multisig.nonce],
        ];
        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;
        solana_program::program::invoke_signed(&ix, &accounts, signer)?;

        // Burn the account to ensure one time use.
        ctx.accounts.transaction.did_execute = true;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateMultisig<'info> {
    #[account(init)]
    multisig: ProgramAccount<'info, Multisig>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CreateTransaction<'info> {
    multisig: ProgramAccount<'info, Multisig>,
    #[account(init)]
    transaction: ProgramAccount<'info, Transaction>,
    #[account(signer)]
    proposer: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Approve<'info> {
    multisig: ProgramAccount<'info, Multisig>,
    #[account(mut, belongs_to = multisig)]
    transaction: ProgramAccount<'info, Transaction>,
    // One of the multisig owners.
    #[account(signer)]
    owner: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Auth<'info> {
    #[account(mut)]
    multisig: ProgramAccount<'info, Multisig>,
    #[account(signer, seeds = [
        multisig.to_account_info().key.as_ref(),
        &[multisig.nonce],
    ])]
    multisig_signer: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ExecuteTransaction<'info> {
    multisig: ProgramAccount<'info, Multisig>,
    #[account(seeds = [
        multisig.to_account_info().key.as_ref(),
        &[multisig.nonce],
    ])]
    multisig_signer: AccountInfo<'info>,
    #[account(mut, belongs_to = multisig)]
    transaction: ProgramAccount<'info, Transaction>,
}

#[account]
pub struct Multisig {
    owners: Vec<Pubkey>,
    threshold: u64,
    nonce: u8,
}

#[account]
pub struct Transaction {
    // Target program to execute against.
    program_id: Pubkey,
    // Accounts requried for the transaction.
    accounts: Vec<TransactionAccount>,
    // Instruction data for the transaction.
    data: Vec<u8>,
    // signers[index] is true iff multisig.owners[index] signed the transaction.
    signers: Vec<bool>,
    // The multisig account this transaction belongs to.
    multisig: Pubkey,
    // Boolean ensuring one time execution.
    did_execute: bool,
}

impl From<&Transaction> for Instruction {
    fn from(tx: &Transaction) -> Instruction {
        Instruction {
            program_id: tx.program_id,
            accounts: tx.accounts.clone().into_iter().map(Into::into).collect(),
            data: tx.data.clone(),
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransactionAccount {
    pubkey: Pubkey,
    is_signer: bool,
    is_writable: bool,
}

impl From<TransactionAccount> for AccountMeta {
    fn from(account: TransactionAccount) -> AccountMeta {
        match account.is_writable {
            false => AccountMeta::new_readonly(account.pubkey, account.is_signer),
            true => AccountMeta::new(account.pubkey, account.is_signer),
        }
    }
}

#[error]
pub enum ErrorCode {
    #[msg("The given owner is not part of this multisig.")]
    InvalidOwner,
    #[msg("Not enough owners signed this transaction.")]
    NotEnoughSigners,
    Unknown,
}
