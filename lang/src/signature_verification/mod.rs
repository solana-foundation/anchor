use crate::pinocchio_runtime::sysvar_instructions::*;
use crate::prelude::*;

mod ed25519;
mod secp256k1;

pub use ed25519::{verify_ed25519_ix, verify_ed25519_ix_with_instruction_index};
pub use secp256k1::{verify_secp256k1_ix, verify_secp256k1_ix_with_instruction_index};

/// Loads the instruction currently executing in this transaction and verifies it
/// as an Ed25519 signature instruction.
pub fn verify_current_ed25519_instruction(
    ix_sysvar: &AccountInfo,
    pubkey: &[u8; 32],
    msg: &[u8],
    sig: &[u8; 64],
) -> Result<()> {
    let instructions = Instructions::try_from(ix_sysvar)?;
    let idx = instructions.load_current_index();
    let ix = instructions
        .load_instruction_at(idx as usize)
        .map_err(|_| error!(error::ErrorCode::ConstraintRaw))?;
    verify_ed25519_ix_with_instruction_index(&ix, idx, pubkey, msg, sig)
}

/// Loads the instruction currently executing in this transaction and verifies it
/// as a Secp256k1 signature instruction.
pub fn verify_current_secp256k1_instruction(
    ix_sysvar: &AccountInfo,
    eth_address: &[u8; 20],
    msg: &[u8],
    sig: &[u8; 64],
    recovery_id: u8,
) -> Result<()> {
    let instructions = Instructions::try_from(ix_sysvar)?;
    let idx_u16 = instructions.load_current_index();
    let idx_u8 =
        u8::try_from(idx_u16).map_err(|_| error!(error::ErrorCode::InvalidNumericConversion))?;
    let ix = instructions.load_instruction_at(idx_u16 as usize)?;
    verify_secp256k1_ix_with_instruction_index(&ix, idx_u8, eth_address, msg, sig, recovery_id)
}
