use crate::error::ErrorCode;
use crate::pinocchio_runtime::sysvar_instructions::IntrospectedInstruction;
use crate::prelude::*;

pub const ID: Pubkey = Pubkey::from_str_const("Ed25519SigVerify111111111111111111111111111");

/// Verifies an Ed25519 signature instruction assuming the signature, public key,
/// and message bytes are embedded directly inside the instruction data (Solana's
/// default encoding). Prefer [`verify_ed25519_ix_with_instruction_index`] when
/// working with custom instructions that point at external instruction data.
pub fn verify_ed25519_ix(
    ix: &IntrospectedInstruction,
    pubkey: &[u8; 32],
    msg: &[u8],
    sig: &[u8; 64],
) -> Result<()> {
    verify_ed25519_ix_with_instruction_index(ix, u16::MAX, pubkey, msg, sig)
}

pub fn verify_ed25519_ix_with_instruction_index(
    ix: &IntrospectedInstruction,
    instruction_index: u16,
    pubkey: &[u8; 32],
    msg: &[u8],
    sig: &[u8; 64],
) -> Result<()> {
    require_keys_eq!(
        ix.get_program_id().key(),
        ID,
        ErrorCode::Ed25519InvalidProgram
    );
    // require_eq!(ix.get_instruction_accounts().len(), 0usize, ErrorCode::InstructionHasAccounts);
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
    expected.extend_from_slice(&instruction_index.to_le_bytes());
    expected.extend_from_slice(&pubkey_offset.to_le_bytes());
    expected.extend_from_slice(&instruction_index.to_le_bytes());
    expected.extend_from_slice(&msg_offset.to_le_bytes());
    expected.extend_from_slice(&msg_len.to_le_bytes());
    expected.extend_from_slice(&instruction_index.to_le_bytes());

    expected.extend_from_slice(sig);
    expected.extend_from_slice(pubkey);
    expected.extend_from_slice(msg);

    if expected != ix.get_instruction_data() {
        return Err(ErrorCode::SignatureVerificationFailed.into());
    }
    Ok(())
}
