use crate::error::ErrorCode;
use crate::prelude::*;
use crate::solana_program::instruction::Instruction;
use solana_sdk_ids::ed25519_program;

/// Verify that `ix.data` is an Ed25519 syscall verifying `sig` over `msg` by `pubkey`.
pub fn verify_ed25519_ix(
    ix: &Instruction,
    pubkey: &[u8; 32],
    msg: &[u8],
    sig: &[u8; 64],
) -> Result<()> {
    require_keys_eq!(
        ix.program_id,
        ed25519_program::id(),
        ErrorCode::Ed25519InvalidProgram
    );
    require_eq!(ix.accounts.len(), 0usize, ErrorCode::InstructionHasAccounts);
    require!(msg.len() <= u16::MAX as usize, ErrorCode::MessageTooLong);

    let num_signatures: u8 = 1;
    let padding: u8 = 0;
    let header_len: usize = 2;
    let offsets_len: usize = 14; // 7 * u16
    let base: usize = header_len + offsets_len;

    let signature_offset: u16 = base as u16;
    let signature_instruction_index: u16 = u16::MAX;
    let public_key_offset: u16 = (base + 64) as u16;
    let public_key_instruction_index: u16 = u16::MAX;
    let message_data_offset: u16 = (base + 64 + 32) as u16; // 32 bytes for pubkey
    let message_data_size: u16 = msg.len() as u16;
    let message_instruction_index: u16 = u16::MAX;

    // [header][offsets][signature][public_key][msg]
    let mut expected = Vec::with_capacity(base + 64 + 32 + msg.len());
    expected.push(num_signatures);
    expected.push(padding);
    expected.extend_from_slice(&signature_offset.to_le_bytes());
    expected.extend_from_slice(&signature_instruction_index.to_le_bytes());
    expected.extend_from_slice(&public_key_offset.to_le_bytes());
    expected.extend_from_slice(&public_key_instruction_index.to_le_bytes());
    expected.extend_from_slice(&message_data_offset.to_le_bytes());
    expected.extend_from_slice(&message_data_size.to_le_bytes());
    expected.extend_from_slice(&message_instruction_index.to_le_bytes());
    expected.extend_from_slice(sig);
    expected.extend_from_slice(pubkey);
    expected.extend_from_slice(msg);

    if ix.data.len() != expected.len() {
        return Err(error!(ErrorCode::DataLengthMismatch));
    }
    if ix.data != expected {
        return Err(error!(ErrorCode::ConstraintRaw));
    }
    Ok(())
}
