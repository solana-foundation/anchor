use crate::prelude::*;
use crate::solana_program::instruction::Instruction;
use core::convert::TryFrom;
use solana_instructions_sysvar::{load_current_index_checked, load_instruction_at_checked};

mod ed25519;
mod secp256k1;

pub use ed25519::{verify_ed25519_ix, verify_ed25519_ix_with_instruction_index};
pub use secp256k1::{verify_secp256k1_ix, verify_secp256k1_ix_with_instruction_index};

/// Load an instruction from the Instructions sysvar at the given index.
pub fn load_instruction(index: usize, ix_sysvar: &AccountInfo<'_>) -> Result<Instruction> {
    let ix = load_instruction_at_checked(index, ix_sysvar)
        .map_err(|_| error!(error::ErrorCode::ConstraintRaw))?;
    Ok(ix)
}

/// Loads the instruction currently executing in this transaction and verifies it
/// as an Ed25519 signature instruction.
pub fn verify_current_ed25519_instruction(
    ix_sysvar: &AccountInfo<'_>,
    pubkey: &[u8; 32],
    msg: &[u8],
    sig: &[u8; 64],
) -> Result<()> {
    let idx = load_current_index_checked(ix_sysvar)
        .map_err(|_| error!(error::ErrorCode::ConstraintRaw))?;
    let ix = load_instruction(idx as usize, ix_sysvar)?;
    verify_ed25519_ix_with_instruction_index(&ix, idx, pubkey, msg, sig)
}

/// Loads the instruction currently executing in this transaction and verifies it
/// as a Secp256k1 signature instruction.
pub fn verify_current_secp256k1_instruction(
    ix_sysvar: &AccountInfo<'_>,
    eth_address: &[u8; 20],
    msg: &[u8],
    sig: &[u8; 64],
    recovery_id: u8,
) -> Result<()> {
    let idx_u16 = load_current_index_checked(ix_sysvar)
        .map_err(|_| error!(error::ErrorCode::ConstraintRaw))?;
    let idx_u8 =
        u8::try_from(idx_u16).map_err(|_| error!(error::ErrorCode::InvalidNumericConversion))?;
    let ix = load_instruction(idx_u16 as usize, ix_sysvar)?;
    verify_secp256k1_ix_with_instruction_index(&ix, idx_u8, eth_address, msg, sig, recovery_id)
}
