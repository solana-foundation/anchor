use crate::error::ErrorCode;
use crate::prelude::*;
use crate::solana_program::instruction::Instruction;
use solana_sdk_ids::secp256k1_program;

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

    const DATA_START: usize = 12; // 1 header + 11 offset bytes
    let eth_len = eth_address.len() as u16;
    let sig_len = sig.len() as u16;
    let msg_len = msg.len() as u16;

    let eth_offset: u16 = DATA_START as u16;
    let sig_offset: u16 = eth_offset + eth_len;
    let msg_offset: u16 = sig_offset + sig_len + 1; // +1 for recovery id

    let mut expected =
        Vec::with_capacity(DATA_START + eth_address.len() + sig.len() + 1 + msg.len());

    expected.push(1u8); // num signatures
    expected.extend_from_slice(&sig_offset.to_le_bytes());
    expected.push(0u8); // sig ix idx
    expected.extend_from_slice(&eth_offset.to_le_bytes());
    expected.push(0u8); // eth ix idx
    expected.extend_from_slice(&msg_offset.to_le_bytes());
    expected.extend_from_slice(&msg_len.to_le_bytes());
    expected.push(0u8); // msg ix idx

    expected.extend_from_slice(eth_address);
    expected.extend_from_slice(sig);
    expected.push(recovery_id);
    expected.extend_from_slice(msg);

    if expected != ix.data {
        return Err(ErrorCode::SignatureVerificationFailed.into());
    }
    Ok(())
}
