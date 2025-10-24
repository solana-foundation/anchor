use crate::prelude::*;
use crate::solana_program::instruction::Instruction;
use crate::solana_program::sysvar::instructions::load_instruction_at_checked;

mod ed25519;
mod secp256k1;

pub use ed25519::verify_ed25519_ix;
pub use secp256k1::verify_secp256k1_ix;

/// Load an instruction from the Instructions sysvar at the given index.
pub fn load_instruction(index: usize, ix_sysvar: &AccountInfo<'_>) -> Result<Instruction> {
    let ix = load_instruction_at_checked(index, ix_sysvar)
        .map_err(|_| error!(error::ErrorCode::ConstraintRaw))?;
    Ok(ix)
}
