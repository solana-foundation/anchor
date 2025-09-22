use crate::error::ErrorCode;
use crate::prelude::*;
use crate::solana_program::instruction::Instruction;
use solana_sdk_ids::secp256k1_program;

/// Verify that `ix.data` is a Secp256k1 syscall verifying `sig` over `msg` by `eth_address` with `recovery_id`.
pub fn verify_secp256k1_ix(
    ix: &Instruction,
    eth_address: &[u8; 20],
    msg: &[u8],
    sig: &[u8; 64],
    recovery_id: u8,
) -> Result<()> {
    require_keys_eq!(
        ix.program_id,
        secp256k1_program::id(),
        ErrorCode::Secp256k1InvalidProgram
    );
    require_eq!(ix.accounts.len(), 0usize, ErrorCode::InstructionHasAccounts);
    require!(recovery_id <= 1, ErrorCode::InvalidRecoveryId);
    require!(msg.len() <= u16::MAX as usize, ErrorCode::MessageTooLong);

    let num_signatures: u8 = 1;
    let num_eth_addresses: u8 = 1;
    let header_len: usize = 2;
    let offsets_len: usize = 18; // 9 * u16
    let base: usize = header_len + offsets_len;

    let signature_offset: u16 = base as u16;
    let signature_instruction_index: u16 = u16::MAX;
    let eth_address_offset: u16 = (base + 64) as u16;
    let eth_address_instruction_index: u16 = u16::MAX;
    let message_data_offset: u16 = (base + 64 + 20) as u16; // 20 bytes for eth_address
    let message_data_size: u16 = msg.len() as u16;
    let message_instruction_index: u16 = u16::MAX;
    let recovery_id_offset: u16 = (base + 64 + 20 + msg.len()) as u16;
    let recovery_id_instruction_index: u16 = u16::MAX;

    // [header][offsets][signature][eth_address][msg][recovery_id]
    let mut expected = Vec::with_capacity(base + 64 + 20 + msg.len() + 1);
    expected.push(num_signatures);
    expected.push(num_eth_addresses);
    expected.extend_from_slice(&signature_offset.to_le_bytes());
    expected.extend_from_slice(&signature_instruction_index.to_le_bytes());
    expected.extend_from_slice(&eth_address_offset.to_le_bytes());
    expected.extend_from_slice(&eth_address_instruction_index.to_le_bytes());
    expected.extend_from_slice(&message_data_offset.to_le_bytes());
    expected.extend_from_slice(&message_data_size.to_le_bytes());
    expected.extend_from_slice(&message_instruction_index.to_le_bytes());
    expected.extend_from_slice(&recovery_id_offset.to_le_bytes());
    expected.extend_from_slice(&recovery_id_instruction_index.to_le_bytes());
    expected.extend_from_slice(sig);
    expected.extend_from_slice(eth_address);
    expected.extend_from_slice(msg);
    expected.push(recovery_id);

    if ix.data.len() != expected.len() {
        return Err(error!(ErrorCode::DataLengthMismatch));
    }
    if ix.data != expected {
        return Err(error!(ErrorCode::ConstraintRaw));
    }
    Ok(())
}
