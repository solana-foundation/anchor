#![no_std]

//! Hand-rolled pinocchio port of the multisig benchmark — the raw-performance
//! floor for this workload. No framework dispatch, no trait machinery. The
//! entrypoint reads a 1-byte instruction discriminator and dispatches directly
//! to one of four hand-written handlers.
//!
//! On-chain state layout (389 bytes):
//!
//! ```text
//!   0    disc: u8 = 1 (MULTISIG_CONFIG_DISC)
//!   1    creator: Address (32 bytes)
//!   33   threshold: u8
//!   34   bump: u8
//!   35   label_len: u8
//!   36   label: [u8; 32]
//!   68   signers_len: u8
//!   69   signers: [Address; 10] (320 bytes)
//!   389  end
//! ```
//!
//! PDAs are derived on-chain with the raw `sol_sha256` +
//! `sol_curve_validate_point` fast path, matching anchor-v2/quasar's cost.

use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::{Seed, Signer as CpiSigner},
    no_allocator, program_entrypoint,
    sysvars::rent::{ACCOUNT_STORAGE_OVERHEAD, DEFAULT_LAMPORTS_PER_BYTE},
    ProgramResult,
};
use solana_program_error::ProgramError;

// Shared multisig program id: `44444444444444444444444444444444444444444444`.
// Matches anchor-v1/anchor-v2/quasar/steel — `find_program_address` returns
// the same bumps across variants so the CU comparison isolates framework cost.
//
// We don't need it as a constant inside the program itself (the entrypoint
// receives `program_id` as a parameter), but the bench driver writes the
// keypair file for this id via `target/deploy/multisig_pinocchio-keypair.json`.
pub const ID: Address = Address::new_from_array([
    0x2d, 0x5b, 0x41, 0x3c, 0x65, 0x40, 0xde, 0x15, 0x0c, 0x93, 0x73, 0x14, 0x4d, 0x51, 0x33, 0xca,
    0x4c, 0xb8, 0x30, 0xba, 0x0f, 0x75, 0x67, 0x16, 0xac, 0xea, 0x0e, 0x50, 0xd7, 0x94, 0x35, 0xe5,
]);

// ---- State layout ----

const MULTISIG_CONFIG_DISC: u8 = 1;

const MAX_LABEL_LEN: usize = 32;
const MAX_SIGNERS: usize = 10;

const OFFSET_DISC: usize = 0;
const OFFSET_CREATOR: usize = 1;
const OFFSET_THRESHOLD: usize = 33;
const OFFSET_BUMP: usize = 34;
const OFFSET_LABEL_LEN: usize = 35;
const OFFSET_LABEL: usize = 36;
const OFFSET_SIGNERS_LEN: usize = OFFSET_LABEL + MAX_LABEL_LEN; // 68
const OFFSET_SIGNERS: usize = OFFSET_SIGNERS_LEN + 1; // 69

pub const MULTISIG_CONFIG_SPACE: usize = OFFSET_SIGNERS + MAX_SIGNERS * 32; // 389

// ---- Instruction discriminators ----

const IX_CREATE: u8 = 0;
const IX_DEPOSIT: u8 = 1;
const IX_SET_LABEL: u8 = 2;
const IX_EXECUTE_TRANSFER: u8 = 3;

// ---- Errors ----

const ERR_INVALID_DATA: u32 = 1;
const ERR_INVALID_THRESHOLD: u32 = 2;
const ERR_TOO_MANY_SIGNERS: u32 = 3;
const ERR_MISSING_SIGNATURE: u32 = 4;
const ERR_LABEL_TOO_LONG: u32 = 5;
const ERR_UNAUTHORIZED_CREATOR: u32 = 6;
const ERR_BAD_PDA: u32 = 7;
const ERR_WRONG_ACCOUNT_OWNER: u32 = 8;
const ERR_ACCOUNT_TOO_SMALL: u32 = 9;

#[inline(always)]
fn custom(code: u32) -> ProgramError {
    ProgramError::Custom(code)
}

// ---- Entrypoint ----

#[cfg(not(feature = "no-entrypoint"))]
program_entrypoint!(process_instruction);
no_allocator!();

#[cfg(all(not(test), target_os = "solana"))]
pinocchio::nostd_panic_handler!();

pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    if instruction_data.is_empty() {
        return Err(custom(ERR_INVALID_DATA));
    }

    let (disc, rest) = instruction_data.split_first().unwrap();
    match *disc {
        IX_CREATE => handle_create(program_id, accounts, rest),
        IX_DEPOSIT => handle_deposit(accounts, rest),
        IX_SET_LABEL => handle_set_label(accounts, rest),
        IX_EXECUTE_TRANSFER => handle_execute_transfer(program_id, accounts, rest),
        _ => Err(custom(ERR_INVALID_DATA)),
    }
}

// ---- create(threshold: u8) ----
//
// Accounts:
//   0: creator (signer, writable)
//   1: config (PDA [b"multisig", creator], writable, uninit)
//   2: system_program
//   3..: additional signer accounts (each must be `is_signer`)

fn handle_create(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    if data.len() < 1 {
        return Err(custom(ERR_INVALID_DATA));
    }
    let threshold = data[0];

    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Scope borrow of the first three accounts so we can later iterate
    // remaining accounts (`accounts[3..]`) for the signer list.
    let creator_addr;
    let config_bump;
    {
        let creator = unsafe { accounts.get_unchecked(0) };
        let config = unsafe { accounts.get_unchecked(1) };

        if !creator.is_signer() {
            return Err(custom(ERR_MISSING_SIGNATURE));
        }

        creator_addr = *creator.address();

        // Derive the config PDA on-chain (must match the address the caller
        // passed in) using the fast `sol_sha256`+`sol_curve_validate_point`
        // path. We also need the bump to sign the CreateAccount CPI.
        let (expected_pda, bump) = find_program_address(
            &[b"multisig", creator_addr.as_ref()],
            program_id,
        )
        .ok_or(custom(ERR_BAD_PDA))?;
        if config.address() != &expected_pda {
            return Err(custom(ERR_BAD_PDA));
        }
        config_bump = bump;

        // CPI: CreateAccount signed with [b"multisig", creator, bump].
        let rent_lamports =
            (ACCOUNT_STORAGE_OVERHEAD + MULTISIG_CONFIG_SPACE as u64) * DEFAULT_LAMPORTS_PER_BYTE;
        let bump_slice = [bump];
        let seeds = [
            Seed::from(b"multisig".as_slice()),
            Seed::from(creator_addr.as_ref()),
            Seed::from(bump_slice.as_slice()),
        ];
        let signer = CpiSigner::from(&seeds[..]);

        pinocchio_system::instructions::CreateAccount {
            from: creator,
            to: config,
            lamports: rent_lamports,
            space: MULTISIG_CONFIG_SPACE as u64,
            owner: program_id,
        }
        .invoke_signed(&[signer])?;
    }

    // Capture signer addresses from remaining accounts (indices 3..).
    let remaining = &accounts[3..];
    if remaining.len() > MAX_SIGNERS {
        return Err(custom(ERR_TOO_MANY_SIGNERS));
    }

    let mut signer_addrs = [[0u8; 32]; MAX_SIGNERS];
    let mut count = 0usize;
    for account in remaining.iter() {
        if !account.is_signer() {
            return Err(custom(ERR_MISSING_SIGNATURE));
        }
        signer_addrs[count] = account.address().to_bytes();
        count = count.wrapping_add(1);
    }

    if threshold == 0 || threshold as usize > count {
        return Err(custom(ERR_INVALID_THRESHOLD));
    }

    // Write the config account data.
    let config = unsafe { accounts.get_unchecked(1) };
    let mut config_mut = *config;
    let data_slice = unsafe { config_mut.borrow_unchecked_mut() };
    if data_slice.len() < MULTISIG_CONFIG_SPACE {
        return Err(custom(ERR_ACCOUNT_TOO_SMALL));
    }
    data_slice[OFFSET_DISC] = MULTISIG_CONFIG_DISC;
    data_slice[OFFSET_CREATOR..OFFSET_CREATOR + 32]
        .copy_from_slice(creator_addr.as_ref());
    data_slice[OFFSET_THRESHOLD] = threshold;
    data_slice[OFFSET_BUMP] = config_bump;
    data_slice[OFFSET_LABEL_LEN] = 0;
    // Label bytes are zero-initialized by CreateAccount.
    data_slice[OFFSET_SIGNERS_LEN] = count as u8;
    for i in 0..count {
        let dst = OFFSET_SIGNERS + i * 32;
        data_slice[dst..dst + 32].copy_from_slice(&signer_addrs[i]);
    }

    Ok(())
}

// ---- deposit(amount: u64) ----
//
// Accounts:
//   0: depositor (signer, writable)
//   1: config (readonly — we only need it to be present in the tx)
//   2: vault (PDA [b"vault", config], writable)
//   3: system_program

fn handle_deposit(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    if data.len() < 8 {
        return Err(custom(ERR_INVALID_DATA));
    }
    let amount = u64::from_le_bytes(data[..8].try_into().unwrap());

    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let depositor = unsafe { accounts.get_unchecked(0) };
    let vault = unsafe { accounts.get_unchecked(2) };

    pinocchio_system::instructions::Transfer {
        from: depositor,
        to: vault,
        lamports: amount,
    }
    .invoke()?;
    Ok(())
}

// ---- set_label(label_len: u8, label: [u8; 32]) ----
//
// Accounts:
//   0: creator (signer, writable)
//   1: config (writable)
//   2: system_program

fn handle_set_label(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    if data.len() < 1 + 32 {
        return Err(custom(ERR_INVALID_DATA));
    }
    let label_len = data[0] as usize;
    if label_len > MAX_LABEL_LEN {
        return Err(custom(ERR_LABEL_TOO_LONG));
    }
    let label_bytes = &data[1..1 + 32];

    // UTF-8 validate the label slice (fairness with frameworks that take a
    // `&str` argument — quasar's `String<32>` does this as part of
    // deserialization).
    core::str::from_utf8(&label_bytes[..label_len])
        .map_err(|_| custom(ERR_LABEL_TOO_LONG))?;

    if accounts.len() < 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let creator = unsafe { accounts.get_unchecked(0) };
    if !creator.is_signer() {
        return Err(custom(ERR_MISSING_SIGNATURE));
    }

    let config = unsafe { accounts.get_unchecked(1) };
    let mut config_mut = *config;
    let cfg_data = unsafe { config_mut.borrow_unchecked_mut() };
    if cfg_data.len() < MULTISIG_CONFIG_SPACE {
        return Err(custom(ERR_ACCOUNT_TOO_SMALL));
    }
    if cfg_data[OFFSET_DISC] != MULTISIG_CONFIG_DISC {
        return Err(custom(ERR_WRONG_ACCOUNT_OWNER));
    }

    // has_one: creator must match the stored creator.
    if &cfg_data[OFFSET_CREATOR..OFFSET_CREATOR + 32] != creator.address().as_ref() {
        return Err(custom(ERR_UNAUTHORIZED_CREATOR));
    }

    cfg_data[OFFSET_LABEL_LEN] = label_len as u8;
    cfg_data[OFFSET_LABEL..OFFSET_LABEL + MAX_LABEL_LEN].copy_from_slice(label_bytes);
    Ok(())
}

// ---- execute_transfer(amount: u64) ----
//
// Accounts:
//   0: config (readonly, PDA [b"multisig", creator])
//   1: creator (readonly)
//   2: vault (writable, PDA [b"vault", config])
//   3: recipient (writable)
//   4: system_program
//   5..: additional signer accounts

fn handle_execute_transfer(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    if data.len() < 8 {
        return Err(custom(ERR_INVALID_DATA));
    }
    let amount = u64::from_le_bytes(data[..8].try_into().unwrap());

    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let config = unsafe { accounts.get_unchecked(0) };
    let creator = unsafe { accounts.get_unchecked(1) };
    let vault = unsafe { accounts.get_unchecked(2) };
    let recipient = unsafe { accounts.get_unchecked(3) };

    // Read the stored signers + threshold from the config account.
    let cfg_data = unsafe { config.borrow_unchecked() };
    if cfg_data.len() < MULTISIG_CONFIG_SPACE {
        return Err(custom(ERR_ACCOUNT_TOO_SMALL));
    }
    if cfg_data[OFFSET_DISC] != MULTISIG_CONFIG_DISC {
        return Err(custom(ERR_WRONG_ACCOUNT_OWNER));
    }

    // has_one: creator must match the stored creator.
    if &cfg_data[OFFSET_CREATOR..OFFSET_CREATOR + 32] != creator.address().as_ref() {
        return Err(custom(ERR_UNAUTHORIZED_CREATOR));
    }

    let threshold = cfg_data[OFFSET_THRESHOLD] as u32;
    let stored_count = cfg_data[OFFSET_SIGNERS_LEN] as usize;
    if stored_count > MAX_SIGNERS {
        return Err(custom(ERR_TOO_MANY_SIGNERS));
    }
    let stored_signers = &cfg_data[OFFSET_SIGNERS..OFFSET_SIGNERS + stored_count * 32];

    // Count approvals from remaining accounts that are `is_signer` and in the
    // stored signers list.
    let remaining = &accounts[5..];
    let mut approvals = 0u32;
    for account in remaining.iter() {
        if !account.is_signer() {
            continue;
        }
        let addr = account.address().as_ref();
        let mut i = 0usize;
        while i < stored_count {
            let start = i * 32;
            if &stored_signers[start..start + 32] == addr {
                approvals = approvals.wrapping_add(1);
                break;
            }
            i += 1;
        }
    }
    if approvals < threshold {
        return Err(custom(ERR_MISSING_SIGNATURE));
    }

    // Derive the vault bump on-chain (matching quasar's approach).
    let config_addr = *config.address();
    let (_vault_pda, vault_bump) =
        find_program_address(&[b"vault", config_addr.as_ref()], program_id)
            .ok_or(custom(ERR_BAD_PDA))?;
    if vault.address() != &_vault_pda {
        return Err(custom(ERR_BAD_PDA));
    }

    let bump_slice = [vault_bump];
    let seeds = [
        Seed::from(b"vault".as_slice()),
        Seed::from(config_addr.as_ref()),
        Seed::from(bump_slice.as_slice()),
    ];
    let signer = CpiSigner::from(&seeds[..]);

    pinocchio_system::instructions::Transfer {
        from: vault,
        to: recipient,
        lamports: amount,
    }
    .invoke_signed(&[signer])?;
    Ok(())
}

// ---- PDA derivation fast path ----
//
// Copied inline from anchor-lang-v2/quasar so this crate stays self-contained.
// Uses raw `sol_sha256` + `sol_curve_validate_point` syscalls for ~3x lower CU
// than `sol_try_find_program_address`.
#[inline(always)]
fn find_program_address(seeds: &[&[u8]], program_id: &Address) -> Option<(Address, u8)> {
    #[cfg(target_os = "solana")]
    {
        use solana_define_syscall::definitions::{sol_curve_validate_point, sol_sha256};

        const CURVE25519_EDWARDS: u64 = 0;
        const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";

        let n = seeds.len();
        if n > 16 {
            return None;
        }

        let mut slices = core::mem::MaybeUninit::<[&[u8]; 19]>::uninit();
        let sptr = slices.as_mut_ptr() as *mut &[u8];

        let mut i = 0;
        while i < n {
            unsafe { sptr.add(i).write(seeds[i]) };
            i += 1;
        }
        unsafe {
            sptr.add(n + 1).write(program_id.as_ref());
            sptr.add(n + 2).write(PDA_MARKER.as_slice());
        }

        let mut bump_arr = [u8::MAX];
        let bump_ptr = bump_arr.as_mut_ptr();
        unsafe { sptr.add(n).write(core::slice::from_raw_parts(bump_ptr, 1)) };

        let input = unsafe { core::slice::from_raw_parts(sptr, n + 3) };
        let mut hash = core::mem::MaybeUninit::<[u8; 32]>::uninit();

        let mut bump: u64 = u8::MAX as u64;
        loop {
            unsafe { bump_ptr.write(bump as u8) };
            unsafe {
                sol_sha256(
                    input as *const _ as *const u8,
                    input.len() as u64,
                    hash.as_mut_ptr() as *mut u8,
                );
            }
            let on_curve = unsafe {
                sol_curve_validate_point(
                    CURVE25519_EDWARDS,
                    hash.as_ptr() as *const u8,
                    core::ptr::null_mut(),
                )
            };
            if on_curve != 0 {
                let hash_bytes = unsafe { hash.assume_init() };
                return Some((Address::new_from_array(hash_bytes), bump as u8));
            }
            if bump == 0 {
                break;
            }
            bump -= 1;
        }
        None
    }

    #[cfg(not(target_os = "solana"))]
    {
        let _ = (seeds, program_id);
        None
    }
}
