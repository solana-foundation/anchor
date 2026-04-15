//! Steel port of the multisig benchmark program.
//!
//! Uses the `steel` crate's account + instruction macros plus `solana_program`
//! as the runtime. State is a fixed-layout `Pod` struct with a 1-byte
//! discriminator written by `create_program_account_with_bump`.
//!
//! On-chain state layout (388 bytes after the 1-byte disc = 389 total):
//!
//! ```text
//!   0    disc: u8 = 1
//!   1    creator: [u8; 32]
//!   33   threshold: u8
//!   34   bump: u8
//!   35   label_len: u8
//!   36   label: [u8; 32]
//!   68   signers_len: u8
//!   69   _pad: [u8; 3]
//!   72   signers: [[u8; 32]; 10]
//!   392  end
//! ```
//!
//! (We pad `signers_len` out to a 4-byte boundary so the `[[u8;32];10]`
//! signers array sits on an aligned offset — `Pod` requires no internal
//! padding bytes.)

use solana_program::{
    entrypoint, msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
};
use steel::*;

declare_id!("44444444444444444444444444444444444444444444");

// --- Errors ---

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MultisigError {
    InvalidThreshold = 1,
    TooManySigners = 2,
    MissingRequiredSignature = 3,
    LabelTooLong = 4,
    UnauthorizedCreator = 5,
    BadPda = 6,
    InvalidInstructionData = 7,
}

impl From<MultisigError> for ProgramError {
    fn from(e: MultisigError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

// --- Account discriminators ---

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MultisigAccount {
    MultisigConfig = 1,
}

impl From<MultisigAccount> for u8 {
    fn from(v: MultisigAccount) -> u8 {
        v as u8
    }
}

// --- State ---

pub const MAX_LABEL_LEN: usize = 32;
pub const MAX_SIGNERS: usize = 10;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MultisigConfig {
    pub creator: [u8; 32],
    pub threshold: u8,
    pub bump: u8,
    pub label_len: u8,
    pub label: [u8; 32],
    pub signers_len: u8,
    pub _pad: [u8; 3],
    pub signers: [[u8; 32]; MAX_SIGNERS],
}

account!(MultisigAccount, MultisigConfig);

// --- Instruction discriminators ---
//
// Steel programs typically use separate instruction structs per opcode, but
// for a straight 1:1 CU comparison we keep the dispatch layout identical to
// pinocchio/quasar: 1-byte disc + LE-packed args.

const IX_CREATE: u8 = 0;
const IX_DEPOSIT: u8 = 1;
const IX_SET_LABEL: u8 = 2;
const IX_EXECUTE_TRANSFER: u8 = 3;

// --- Entrypoint ---

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (disc, rest) = instruction_data
        .split_first()
        .ok_or(MultisigError::InvalidInstructionData)?;

    match *disc {
        IX_CREATE => handle_create(program_id, accounts, rest),
        IX_DEPOSIT => handle_deposit(accounts, rest),
        IX_SET_LABEL => handle_set_label(accounts, rest),
        IX_EXECUTE_TRANSFER => handle_execute_transfer(program_id, accounts, rest),
        _ => {
            msg!("bad disc");
            Err(MultisigError::InvalidInstructionData.into())
        }
    }
}

// --- create(threshold: u8) ---
//
// Accounts:
//   0: creator (signer, writable)
//   1: config (PDA [b"multisig", creator], writable, uninit)
//   2: system_program
//   3..: additional signer accounts (each must be `is_signer`)

fn handle_create(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if data.is_empty() {
        return Err(MultisigError::InvalidInstructionData.into());
    }
    let threshold = data[0];

    let [creator, config, system_program, rest @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !creator.is_signer {
        return Err(MultisigError::MissingRequiredSignature.into());
    }

    // Derive the config PDA bump and then use steel's
    // `create_program_account_with_bump` to avoid a redundant derivation.
    let (expected, bump) =
        Pubkey::find_program_address(&[b"multisig", creator.key.as_ref()], program_id);
    if config.key != &expected {
        return Err(MultisigError::BadPda.into());
    }

    create_program_account_with_bump::<MultisigConfig>(
        config,
        system_program,
        creator,
        program_id,
        &[b"multisig", creator.key.as_ref()],
        bump,
    )?;

    // Capture signer addresses.
    if rest.len() > MAX_SIGNERS {
        return Err(MultisigError::TooManySigners.into());
    }
    let mut signers_tmp = [[0u8; 32]; MAX_SIGNERS];
    let mut count = 0usize;
    for a in rest.iter() {
        if !a.is_signer {
            return Err(MultisigError::MissingRequiredSignature.into());
        }
        signers_tmp[count] = a.key.to_bytes();
        count += 1;
    }

    if threshold == 0 || threshold as usize > count {
        return Err(MultisigError::InvalidThreshold.into());
    }

    // Populate the newly-allocated account. Discriminator is written by
    // `create_program_account_with_bump`; the payload starts at offset 8.
    let mut raw = config.data.borrow_mut();
    let cfg = bytemuck::from_bytes_mut::<MultisigConfig>(
        &mut raw[8..8 + core::mem::size_of::<MultisigConfig>()],
    );
    cfg.creator = creator.key.to_bytes();
    cfg.threshold = threshold;
    cfg.bump = bump;
    cfg.label_len = 0;
    cfg.label = [0u8; 32];
    cfg.signers_len = count as u8;
    cfg._pad = [0u8; 3];
    cfg.signers[..count].copy_from_slice(&signers_tmp[..count]);

    Ok(())
}

// --- deposit(amount: u64) ---
//
// Accounts:
//   0: depositor (signer, writable)
//   1: config (readonly — just needs to be present for PDA consistency)
//   2: vault (writable, PDA [b"vault", config])
//   3: system_program

fn handle_deposit(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 8 {
        return Err(MultisigError::InvalidInstructionData.into());
    }
    let amount = u64::from_le_bytes(data[..8].try_into().unwrap());

    let [depositor, _config, vault, system_program, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    invoke(
        &system_instruction::transfer(depositor.key, vault.key, amount),
        &[depositor.clone(), vault.clone(), system_program.clone()],
    )?;
    Ok(())
}

// --- set_label(label_len: u8, label: [u8; 32]) ---
//
// Accounts:
//   0: creator (signer, writable)
//   1: config (writable)
//   2: system_program

fn handle_set_label(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 1 + 32 {
        return Err(MultisigError::InvalidInstructionData.into());
    }
    let label_len = data[0] as usize;
    if label_len > MAX_LABEL_LEN {
        return Err(MultisigError::LabelTooLong.into());
    }
    let label_bytes: [u8; 32] = data[1..1 + 32].try_into().unwrap();

    // UTF-8 validate — fairness with frameworks that take a `&str` argument
    // (e.g. quasar's `String<32>`).
    core::str::from_utf8(&label_bytes[..label_len])
        .map_err(|_| MultisigError::LabelTooLong)?;

    let [creator, config, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    if !creator.is_signer {
        return Err(MultisigError::MissingRequiredSignature.into());
    }

    let mut raw = config.data.borrow_mut();
    if raw.len() < 8 + core::mem::size_of::<MultisigConfig>() {
        return Err(ProgramError::InvalidAccountData);
    }
    if raw[0] != MultisigAccount::MultisigConfig as u8 {
        return Err(ProgramError::InvalidAccountData);
    }
    let cfg = bytemuck::from_bytes_mut::<MultisigConfig>(
        &mut raw[8..8 + core::mem::size_of::<MultisigConfig>()],
    );

    // has_one: creator must match stored creator.
    if cfg.creator != creator.key.to_bytes() {
        return Err(MultisigError::UnauthorizedCreator.into());
    }

    cfg.label_len = label_len as u8;
    cfg.label = label_bytes;
    Ok(())
}

// --- execute_transfer(amount: u64) ---
//
// Accounts:
//   0: config (readonly, PDA [b"multisig", creator])
//   1: creator (readonly)
//   2: vault (writable, PDA [b"vault", config])
//   3: recipient (writable)
//   4: system_program
//   5..: additional signer accounts

fn handle_execute_transfer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if data.len() < 8 {
        return Err(MultisigError::InvalidInstructionData.into());
    }
    let amount = u64::from_le_bytes(data[..8].try_into().unwrap());

    let [config, creator, vault, recipient, system_program, rest @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Read the stored signers + threshold from config.
    let raw = config.data.borrow();
    if raw.len() < 8 + core::mem::size_of::<MultisigConfig>() {
        return Err(ProgramError::InvalidAccountData);
    }
    if raw[0] != MultisigAccount::MultisigConfig as u8 {
        return Err(ProgramError::InvalidAccountData);
    }
    let cfg = bytemuck::from_bytes::<MultisigConfig>(
        &raw[8..8 + core::mem::size_of::<MultisigConfig>()],
    );

    if cfg.creator != creator.key.to_bytes() {
        return Err(MultisigError::UnauthorizedCreator.into());
    }

    let threshold = cfg.threshold as u32;
    let stored_count = cfg.signers_len as usize;
    if stored_count > MAX_SIGNERS {
        return Err(MultisigError::TooManySigners.into());
    }

    let mut approvals = 0u32;
    for a in rest.iter() {
        if !a.is_signer {
            continue;
        }
        let addr = a.key.to_bytes();
        let mut i = 0usize;
        while i < stored_count {
            if cfg.signers[i] == addr {
                approvals += 1;
                break;
            }
            i += 1;
        }
    }
    if approvals < threshold {
        return Err(MultisigError::MissingRequiredSignature.into());
    }

    // Derive the vault PDA on-chain, matching quasar/pinocchio behavior.
    let config_key = *config.key;
    // Drop the immutable borrow of config data before the CPI (CPI will
    // borrow accounts reentrantly via the runtime).
    drop(raw);

    let (expected_vault, vault_bump) =
        Pubkey::find_program_address(&[b"vault", config_key.as_ref()], program_id);
    if vault.key != &expected_vault {
        return Err(MultisigError::BadPda.into());
    }

    let bump_arr = [vault_bump];
    let seeds: &[&[u8]] = &[b"vault", config_key.as_ref(), &bump_arr];
    invoke_signed(
        &system_instruction::transfer(vault.key, recipient.key, amount),
        &[vault.clone(), recipient.clone(), system_program.clone()],
        &[seeds],
    )?;
    Ok(())
}
