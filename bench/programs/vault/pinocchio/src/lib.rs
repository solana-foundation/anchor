#![no_std]

//! Hand-rolled pinocchio SOL vault — the perf floor for this bench.
//! `deposit` CPIs `system::Transfer`; `withdraw` mutates lamports directly
//! on a program-owned PDA.

use pinocchio::{
    account::AccountView, address::Address, no_allocator, program_entrypoint, ProgramResult,
};
use solana_program_error::ProgramError;

// Shared vault id — base58 `33333...` (44 threes). Must stay byte-identical
// across all five vault variants so the `[b"vault", user]` PDA matches.
// The line below is scanned by `anchor debugger`'s program discovery to map
// this .so's runtime id without depending on anchor's `declare_id!` macro.
// declare_id!("33333333333333333333333333333333333333333333")
pub const ID: Address = Address::new_from_array([
    0x1e, 0x3c, 0xd6, 0x28, 0x43, 0x80, 0x94, 0x0e, 0x08, 0x62, 0x4c, 0xb8, 0x33, 0x8b, 0x77, 0xdc,
    0x33, 0x25, 0x75, 0xd1, 0x5f, 0xa3, 0x9a, 0x0f, 0x1d, 0xf1, 0x5e, 0xe0, 0x8f, 0xb8, 0x23, 0xee,
]);

const IX_DEPOSIT: u8 = 0;
const IX_WITHDRAW: u8 = 1;

const ERR_INVALID_DATA: u32 = 1;

#[inline(always)]
fn custom(code: u32) -> ProgramError {
    ProgramError::Custom(code)
}

#[cfg(not(feature = "no-entrypoint"))]
program_entrypoint!(process_instruction);
no_allocator!();

#[cfg(all(not(test), target_os = "solana"))]
pinocchio::nostd_panic_handler!();

pub fn process_instruction(
    _program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let (disc, rest) = instruction_data
        .split_first()
        .ok_or(custom(ERR_INVALID_DATA))?;

    match *disc {
        IX_DEPOSIT => handle_deposit(accounts, rest),
        IX_WITHDRAW => handle_withdraw(accounts, rest),
        _ => Err(custom(ERR_INVALID_DATA)),
    }
}

fn parse_amount(data: &[u8]) -> Result<u64, ProgramError> {
    if data.len() < 8 {
        return Err(custom(ERR_INVALID_DATA));
    }
    Ok(u64::from_le_bytes(data[..8].try_into().unwrap()))
}

fn handle_deposit(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let amount = parse_amount(data)?;

    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let user = unsafe { accounts.get_unchecked(0) };
    let vault = unsafe { accounts.get_unchecked(1) };

    pinocchio_system::instructions::Transfer {
        from: user,
        to: vault,
        lamports: amount,
    }
    .invoke()?;
    Ok(())
}

fn handle_withdraw(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let amount = parse_amount(data)?;

    if accounts.len() < 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    // AccountView is Copy — `*accounts[n]` gives an owned handle with write
    // access to the underlying runtime buffer, same pattern as v2's withdraw.
    let mut user = unsafe { *accounts.get_unchecked(0) };
    let mut vault = unsafe { *accounts.get_unchecked(1) };

    vault.set_lamports(vault.lamports() - amount);
    user.set_lamports(user.lamports() + amount);
    Ok(())
}
