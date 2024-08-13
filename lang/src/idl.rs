//! Defines the instructions and account state used to store a program's
//! IDL on-chain at a canonical account address, which can be derived as a
//! function of nothing other than the program's ID.
//!
//! It can be upgraded in a way similar to a BPF upgradeable program. That is,
//! one may invoke the `IdlInstruction::CreateBuffer` instruction to create
//! a buffer, `IdlInstruction::Write` to write a new IDL into it, and then
//! `IdlInstruction::SetBuffer` to copy the IDL into the program's canonical
//! IDL account. In order to perform this upgrade, the buffer's `authority`
//! must match the canonical IDL account's authority.
//!
//! Because the IDL can be larger than the max transaction size, the transaction
//! must be broken up into several pieces and stored into the IDL account with
//! multiple transactions via the `Write` instruction to continuously append to
//! the account's IDL data buffer.
//!
//! Note that IDL account instructions are automatically inserted into all
//! Anchor programs. To remove them, one can use the `no-idl` feature.

use crate::prelude::*;

// The first 8 bytes of an instruction to create or modify the IDL account. This
// instruction is defined outside the main program's instruction enum, so that
// the enum variant tags can align with function source order.
//
// Sha256(anchor:idl)[..8];
pub const IDL_IX_TAG: u64 = 0x0a69e9a778bcf440;
pub const IDL_IX_TAG_LE: &[u8] = IDL_IX_TAG.to_le_bytes().as_slice();

// The Pubkey that is stored as the 'authority' on the IdlAccount when the authority
// is "erased".
pub const ERASED_AUTHORITY: Pubkey = Pubkey::new_from_array([0u8; 32]);

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum IdlInstruction {
    // One time initializer for creating the program's idl account.
    Create { data_len: u64 },
    // Creates a new IDL account buffer. Can be called several times.
    CreateBuffer,
    // Appends the given data to the end of the idl account buffer.
    Write { data: Vec<u8> },
    // Sets a new data buffer for the IdlAccount.
    SetBuffer,
    // Sets a new authority on the IdlAccount.
    SetAuthority { new_authority: Pubkey },
    Close,
    // Increases account size for accounts that need over 10kb.
    Resize { data_len: u64 },
}

// The account holding a program's IDL. This is stored on chain so that clients
// can fetch it and generate a client with nothing but a program's ID.
//
// Note: we use the same account for the "write buffer", similar to the
//       bpf upgradeable loader's mechanism.
//
// TODO: IdlAccount exists here only because it's needed by the CLI, the IDL
// itself uses an IdlAccount defined inside the program itself, see program/idl.rs.
// Ideally it would be deleted and a better solution for sharing the type with CLI
// could be found.
#[account("internal")]
#[derive(Debug)]
pub struct IdlAccount {
    // Address that can modify the IDL.
    pub authority: Pubkey,
    // Length of compressed idl bytes.
    pub data_len: u32,
    // Followed by compressed idl bytes.
}

impl IdlAccount {
    pub fn address(program_id: &Pubkey) -> Pubkey {
        let program_signer = Pubkey::find_program_address(&[], program_id).0;
        Pubkey::create_with_seed(&program_signer, IdlAccount::seed(), program_id)
            .expect("Seed is always valid")
    }
    pub fn seed() -> &'static str {
        "anchor:idl"
    }
}

#[cfg(feature = "idl-build")]
pub use anchor_lang_idl::{build::IdlBuild, *};
