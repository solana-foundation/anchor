//! idl.rs defines the instructions and account state used to store a
//! program's IDL.
//!
//! Note that the transaction to store the IDL can be larger than the max
//! transaction size. As a reuslt, the transaction must be broken up into
//! several pieces and stored into the IDL account with multiple transactions
//! via the `Write` instruction to continuously append to the account's IDL data
//! buffer.
//!
//! To upgrade the IDL, first invoke the `Clear` instruction to reset the data.
//! And invoke `Write` once more. To eliminate the ability to change the IDL,
//! set the authority to a key for which you can't sign, e.g., the zero address
//! or the system program ID, or compile the program with the "no-idl" feature
//! and upgrade the program with the upgradeable BPF loader.

use crate::prelude::*;
use solana_program::pubkey::Pubkey;

// The first 8 bytes of an instruction to create or modify the IDL account. This
// instruction is defined outside the main program's instruction enum, so that
// the enum variant tags can align with function source order.
//
// Sha256(anchor:idl)[..8];
pub const IDL_IX_TAG: u64 = 0x0a69e9a778bcf440;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum IdlInstruction {
    // One time initializer for creating the program's idl account.
    Create { data_len: u64 },
    // Appends to the end of the idl account data.
    Write { data: Vec<u8> },
    // Clear's the IdlInstruction data. Used to update the IDL.
    Clear,
    // Sets a new authority on the IdlAccount.
    SetAuthority { new_authority: Pubkey },
}

// Accounts for the Create instuction.
pub type IdlCreateAccounts<'info> = crate::ctor::Ctor<'info>;

// Accounts for Idl instructions.
#[derive(Accounts)]
pub struct IdlAccounts<'info> {
    #[account(mut, has_one = authority)]
    pub idl: ProgramAccount<'info, IdlAccount>,
    #[account(signer, "authority.key != &Pubkey::new_from_array([0u8; 32])")]
    pub authority: AccountInfo<'info>,
}

// The account holding a program's IDL. This is stored on chain so that clients
// can fetch it and generate a client with nothing but a program's ID.
#[account]
#[derive(Debug)]
pub struct IdlAccount {
    // Address that can modify the IDL.
    pub authority: Pubkey,
    // Compressed idl bytes.
    pub data: Vec<u8>,
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
