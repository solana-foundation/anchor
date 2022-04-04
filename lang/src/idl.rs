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

#[allow(deprecated)]
use crate::accounts::program_account::ProgramAccount;
use crate::prelude::*;
use solana_program::pubkey::Pubkey;

// The first 8 bytes of an instruction to create or modify the IDL account. This
// instruction is defined outside the main program's instruction enum, so that
// the enum variant tags can align with function source order.
//
// Sha256(anchor:idl)[..8];
pub const IDL_IX_TAG: u64 = 0x0a69e9a778bcf440;

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
}

// Accounts for the Create instruction.
pub type IdlCreateAccounts<'info> = crate::ctor::Ctor<'info>;

// Accounts for Idl instructions.
#[derive(Accounts)]
pub struct IdlAccounts<'info> {
    #[account(mut, has_one = authority)]
    #[allow(deprecated)]
    pub idl: ProgramAccount<'info, IdlAccount>,
    #[account(constraint = authority.key != &ERASED_AUTHORITY)]
    pub authority: Signer<'info>,
}

// Accounts for creating an idl buffer.
#[derive(Accounts)]
pub struct IdlCreateBuffer<'info> {
    #[account(zero)]
    #[allow(deprecated)]
    pub buffer: ProgramAccount<'info, IdlAccount>,
    #[account(constraint = authority.key != &ERASED_AUTHORITY)]
    pub authority: Signer<'info>,
}

// Accounts for upgrading the canonical IdlAccount with the buffer.
#[derive(Accounts)]
pub struct IdlSetBuffer<'info> {
    // The buffer with the new idl data.
    #[account(mut, constraint = buffer.authority == idl.authority)]
    #[allow(deprecated)]
    pub buffer: ProgramAccount<'info, IdlAccount>,
    // The idl account to be updated with the buffer's data.
    #[account(mut, has_one = authority)]
    #[allow(deprecated)]
    pub idl: ProgramAccount<'info, IdlAccount>,
    #[account(constraint = authority.key != &ERASED_AUTHORITY)]
    pub authority: Signer<'info>,
}

// The account holding a program's IDL. This is stored on chain so that clients
// can fetch it and generate a client with nothing but a program's ID.
//
// Note: we use the same account for the "write buffer", similar to the
//       bpf upgradeable loader's mechanism.
#[account("internal")]
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
