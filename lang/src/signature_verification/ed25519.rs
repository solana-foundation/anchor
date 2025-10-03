use crate::error::ErrorCode;
use crate::prelude::*;
use crate::solana_program::instruction::Instruction;
use solana_sdk_ids::ed25519_program;

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

    const DATA_START: usize = 16; // 2 header + 14 offset bytes
    let pubkey_len = pubkey.len() as u16;
    let sig_len = sig.len() as u16;
    let msg_len = msg.len() as u16;

    let sig_offset: u16 = DATA_START as u16;
    let pubkey_offset: u16 = sig_offset + sig_len;
    let msg_offset: u16 = pubkey_offset + pubkey_len;

    let mut expected = Vec::with_capacity(DATA_START + sig.len() + pubkey.len() + msg.len());

    expected.push(1u8); // num signatures
    expected.push(0u8); // padding
    expected.extend_from_slice(&sig_offset.to_le_bytes());
    expected.extend_from_slice(&(u16::MAX as u16).to_le_bytes()); 
    expected.extend_from_slice(&pubkey_offset.to_le_bytes());
    expected.extend_from_slice(&(u16::MAX as u16).to_le_bytes()); 
    expected.extend_from_slice(&msg_offset.to_le_bytes());
    expected.extend_from_slice(&msg_len.to_le_bytes());
    expected.extend_from_slice(&(u16::MAX as u16).to_le_bytes()); 

    expected.extend_from_slice(sig);
    expected.extend_from_slice(pubkey);
    expected.extend_from_slice(msg);

    if expected != ix.data {
        return Err(ErrorCode::SignatureVerificationFailed.into());
    }
    Ok(())
}
