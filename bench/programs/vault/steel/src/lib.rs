//! Steel port of the SOL vault. `deposit` CPIs `system::transfer`;
//! `withdraw` mutates lamports directly on a program-owned vault PDA.

use solana_program::{
    entrypoint,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
};
use steel::*;

declare_id!("33333333333333333333333333333333333333333333");

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VaultError {
    InvalidInstructionData = 1,
}

impl From<VaultError> for ProgramError {
    fn from(e: VaultError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

const IX_DEPOSIT: u8 = 0;
const IX_WITHDRAW: u8 = 1;

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (disc, rest) = instruction_data
        .split_first()
        .ok_or(VaultError::InvalidInstructionData)?;

    match *disc {
        IX_DEPOSIT => handle_deposit(accounts, rest),
        IX_WITHDRAW => handle_withdraw(accounts, rest),
        _ => Err(VaultError::InvalidInstructionData.into()),
    }
}

fn parse_amount(data: &[u8]) -> Result<u64, ProgramError> {
    if data.len() < 8 {
        return Err(VaultError::InvalidInstructionData.into());
    }
    Ok(u64::from_le_bytes(data[..8].try_into().unwrap()))
}

fn handle_deposit(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let amount = parse_amount(data)?;

    let [user, vault, system_program, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    invoke(
        &system_instruction::transfer(user.key, vault.key, amount),
        &[user.clone(), vault.clone(), system_program.clone()],
    )?;
    Ok(())
}

fn handle_withdraw(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let amount = parse_amount(data)?;

    let [user, vault, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    **vault.try_borrow_mut_lamports()? -= amount;
    **user.try_borrow_mut_lamports()? += amount;
    Ok(())
}
