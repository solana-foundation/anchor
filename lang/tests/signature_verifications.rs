use anchor_lang::signature_verifications::{verify_ed25519_ix, verify_secp256k1_ix};
use anchor_lang::solana_program::instruction::Instruction;
use solana_sdk_ids::{ed25519_program, secp256k1_program};

#[test]
fn test_verify_ed25519_matches() {
    let pubkey = [7u8; 32];
    let msg = b"Testing #3944 with Ed25519 Signature".to_vec();
    let sig = [9u8; 64];

    let num_signatures: u8 = 1;
    let padding: u8 = 0;
    let header_len: usize = 2;
    let offsets_len: usize = 14;
    let base: usize = header_len + offsets_len;

    let signature_offset: u16 = base as u16;
    let signature_instruction_index: u16 = u16::MAX;
    let public_key_offset: u16 = (base + 64) as u16;
    let public_key_instruction_index: u16 = u16::MAX;
    let message_data_offset: u16 = (base + 64 + 32) as u16;
    let message_data_size: u16 = msg.len() as u16;
    let message_instruction_index: u16 = u16::MAX;

    let mut data = Vec::with_capacity(base + 64 + 32 + msg.len());
    data.push(num_signatures);
    data.push(padding);
    data.extend_from_slice(&signature_offset.to_le_bytes());
    data.extend_from_slice(&signature_instruction_index.to_le_bytes());
    data.extend_from_slice(&public_key_offset.to_le_bytes());
    data.extend_from_slice(&public_key_instruction_index.to_le_bytes());
    data.extend_from_slice(&message_data_offset.to_le_bytes());
    data.extend_from_slice(&message_data_size.to_le_bytes());
    data.extend_from_slice(&message_instruction_index.to_le_bytes());
    data.extend_from_slice(&sig);
    data.extend_from_slice(&pubkey);
    data.extend_from_slice(&msg);

    let ix = Instruction {
        program_id: ed25519_program::id(),
        accounts: vec![],
        data,
    };
    assert!(verify_ed25519_ix(&ix, &pubkey, &msg, &sig).is_ok());
}

#[test]
fn test_verify_ed25519_mismatch() {
    let pubkey = [7u8; 32];
    let msg = b"Testing #3944 with Ed25519 Signature".to_vec();
    let sig = [9u8; 64];

    let data = vec![0u8; 10];
    let ix = Instruction {
        program_id: ed25519_program::id(),
        accounts: vec![],
        data,
    };
    assert!(verify_ed25519_ix(&ix, &pubkey, &msg, &sig).is_err());
}

#[test]
fn test_verify_secp256k1_matches() {
    let eth_address = [0x11u8; 20];
    let msg = b"Testing #3944 with Secp256k1 Signature".to_vec();
    let sig = [0x22u8; 64];
    let recovery_id: u8 = 1;

    let num_signatures: u8 = 1;
    let num_eth_addresses: u8 = 1;
    let header_len: usize = 2;
    let offsets_len: usize = 18;
    let base: usize = header_len + offsets_len;

    let signature_offset: u16 = base as u16;
    let signature_instruction_index: u16 = u16::MAX;
    let eth_address_offset: u16 = (base + 64) as u16;
    let eth_address_instruction_index: u16 = u16::MAX;
    let message_data_offset: u16 = (base + 64 + 20) as u16;
    let message_data_size: u16 = msg.len() as u16;
    let message_instruction_index: u16 = u16::MAX;
    let recovery_id_offset: u16 = (base + 64 + 20 + msg.len()) as u16;
    let recovery_id_instruction_index: u16 = u16::MAX;

    let mut data = Vec::with_capacity(base + 64 + 20 + msg.len() + 1);
    data.push(num_signatures);
    data.push(num_eth_addresses);
    data.extend_from_slice(&signature_offset.to_le_bytes());
    data.extend_from_slice(&signature_instruction_index.to_le_bytes());
    data.extend_from_slice(&eth_address_offset.to_le_bytes());
    data.extend_from_slice(&eth_address_instruction_index.to_le_bytes());
    data.extend_from_slice(&message_data_offset.to_le_bytes());
    data.extend_from_slice(&message_data_size.to_le_bytes());
    data.extend_from_slice(&message_instruction_index.to_le_bytes());
    data.extend_from_slice(&recovery_id_offset.to_le_bytes());
    data.extend_from_slice(&recovery_id_instruction_index.to_le_bytes());
    data.extend_from_slice(&sig);
    data.extend_from_slice(&eth_address);
    data.extend_from_slice(&msg);
    data.push(recovery_id);

    let ix = Instruction {
        program_id: secp256k1_program::id(),
        accounts: vec![],
        data,
    };
    assert!(verify_secp256k1_ix(&ix, &eth_address, &msg, &sig, recovery_id).is_ok());
}

#[test]
fn test_verify_secp256k1_mismatch() {
    let eth_address = [0x11u8; 20];
    let msg = b"Testing #3944 with Secp256k1 Signature".to_vec();
    let sig = [0x22u8; 64];
    let recovery_id: u8 = 1;

    let ix = Instruction {
        program_id: secp256k1_program::id(),
        accounts: vec![],
        data: vec![1, 2, 3],
    };
    assert!(verify_secp256k1_ix(&ix, &eth_address, &msg, &sig, recovery_id).is_err());
}
