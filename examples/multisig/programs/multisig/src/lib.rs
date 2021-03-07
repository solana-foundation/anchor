//! An example of a multisig to execute arbitrary Solana transactions.
//!
//! This program can be used to allow a multisig to govern anything a regular
//! Pubkey can govern. One can use the multisig as a BPF program upgrade
//! authority, a mint authority, etc.
//!
//! To use, one must first create a `Multisig` account, sepcifying two important
//! parameters:
//!
//! 1. Owners - the set of addresses that sign transactions for the multisig.
//! 2. Threhsold - the number of signers required to execute a transaction.
//!
//! Once the `Multisig` account is created, once can create a `Transaction`
//! account, specifying the parameters for a normal solana transaction.
//!
//! To sign, owners shold invoke the `approve` instruction, and finally,
//! the `execute_transaction`, once enough (i.e. `threhsold`) of the owners have
//! signed.

#![feature(proc_macro_hygiene)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_lang::solana_program::instruction::Instruction;
use std::convert::Into;

#[program]
pub mod multisig {
    use super::*;

    // Initializes a new multisig account with a set of owners and a threshold.
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

    // Creates a new transaction account, automatically signed by the creator,
    // which must be one of the owners of the multisig.
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

    // Deletes a transaction if no one has signed, other than the creator.
    // Note that the account is not actually deleted until the next epoch,
    // when the account is garbage collected since there's no more SOL left.
    // As a result, during the period of time when the transaction is deleted
    // but the next epoch hasn't occured, the transaction account can still
    // be used. To get around this, one could just add a field to mark deletion.
    // This is not currently done.
    pub fn delete_transaction(ctx: Context<DeleteTransaction>) -> Result<()> {
        // Check the creator is the one and only signer.
        let num_signers = ctx
            .accounts
            .transaction
            .signers
            .iter()
            .fold(0, |acc, did_sign| if *did_sign { acc + 1 } else { acc });
        if num_signers != 1 {
            return Err(ErrorCode::TransactionAlreadySigned.into());
        }
        let owner_index = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a == ctx.accounts.owner.key)
            .ok_or(ErrorCode::InvalidOwner)?;
        if !ctx.accounts.transaction.signers[owner_index] {
            return Err(ErrorCode::UnableToDelete.into());
        }

        // Mark the creator as not signed.
        ctx.accounts.transaction.signers[owner_index] = false;

        // Send the rent exemption sol back to the creator of the transaction.
        let tx_lamports = ctx.accounts.transaction.to_account_info().lamports();
        let to_lamports = ctx.accounts.owner.lamports();
        **ctx.accounts.owner.lamports.borrow_mut() = to_lamports
            .checked_add(tx_lamports)
            .ok_or(ErrorCode::Overflow)?;
        **ctx
            .accounts
            .transaction
            .to_account_info()
            .lamports
            .borrow_mut() = 0;

        Ok(())
    }

    // Approves a transaction on behalf of an owner of the multisig.
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

    // Changes the execution threshold of the multisig. The only way this can be
    // invoked is via a recursive call from execute_transaction ->
    // change_threshold.
    pub fn change_threshold(ctx: Context<Auth>, threshold: u64) -> Result<()> {
        if threshold > ctx.accounts.multisig.owners.len() as u64 {
            return Err(ErrorCode::InvalidThreshold.into());
        }
        let multisig = &mut ctx.accounts.multisig;
        multisig.threshold = threshold;
        Ok(())
    }

    // Executes the given transaction if threshold owners have signed it.
    pub fn execute_transaction(ctx: Context<ExecuteTransaction>) -> Result<()> {
        // Has this been executed already?
        if ctx.accounts.transaction.did_execute {
            return Err(ErrorCode::AlreadyExecuted.into());
        }

        // Do we have enough signers.
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

        // Execute the transaction signed by the multisig.
        let mut ix: Instruction = (&*ctx.accounts.transaction).into();
        ix.accounts = ix
            .accounts
            .iter()
            .map(|acc| {
                if &acc.pubkey == ctx.accounts.multisig_signer.key {
                    AccountMeta::new_readonly(acc.pubkey, true)
                } else {
                    acc.clone()
                }
            })
            .collect();
        let seeds = &[
            ctx.accounts.multisig.to_account_info().key.as_ref(),
            &[ctx.accounts.multisig.nonce],
        ];
        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;
        solana_program::program::invoke_signed(&ix, &accounts, signer)?;

        // Burn the transaction to ensure one time use.
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
    // One of the owners. Checked in the handler.
    #[account(signer)]
    proposer: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct DeleteTransaction<'info> {
    multisig: ProgramAccount<'info, Multisig>,
    #[account(belongs_to = multisig)]
    transaction: ProgramAccount<'info, Transaction>,
    // One of the multisig owners. Checked in the handler.
    #[account(signer)]
    owner: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Approve<'info> {
    multisig: ProgramAccount<'info, Multisig>,
    #[account(mut, belongs_to = multisig)]
    transaction: ProgramAccount<'info, Transaction>,
    // One of the multisig owners. Checked in the handler.
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
    // The multisig account this transaction belongs to.
    multisig: Pubkey,
    // Target program to execute against.
    program_id: Pubkey,
    // Accounts requried for the transaction.
    accounts: Vec<TransactionAccount>,
    // Instruction data for the transaction.
    data: Vec<u8>,
    // signers[index] is true iff multisig.owners[index] signed the transaction.
    signers: Vec<bool>,
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
    #[msg("Cannot delete a transaction that has been signed by an owner.")]
    TransactionAlreadySigned,
    #[msg("Overflow when adding.")]
    Overflow,
    #[msg("Cannot delete a transaction the owner did not create.")]
    UnableToDelete,
    #[msg("The given transaction has already been executed.")]
    AlreadyExecuted,
    #[msg("Threshold must be less than or equal to the number of owners.")]
    InvalidThreshold,
}
