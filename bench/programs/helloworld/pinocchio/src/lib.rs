//! Hand-optimized pinocchio counter — the performance floor for this benchmark.
//!
//! No framework dispatch, no trait machinery. The entrypoint runs directly:
//! derive the counter PDA on-chain (fair comparison with v2/quasar/v1/steel
//! which all pay the same `find_program_address` cost inside their init
//! macros), CPI-create the account, and write the discriminator + initial
//! value + bump directly through the raw account data pointer.
//!
//! We use `anchor_lang_v2::{find_program_address, create_account_signed}`
//! only as utility functions — these are thin pinocchio wrappers, not
//! framework macros, and they're the same helpers v2 calls internally.

use {
    anchor_lang_v2::{create_account_signed, find_program_address},
    pinocchio::{
        account::AccountView, address::Address, no_allocator, program_entrypoint, ProgramResult,
    },
};

// Same id as every other helloworld variant (base58: B7ihZyo...).
pub const ID: Address = Address::new_from_array([
    150, 77, 128, 252, 18, 209, 27, 135, 162, 60, 31, 212, 195, 62, 83, 235,
    169, 66, 250, 246, 54, 206, 179, 35, 44, 128, 206, 68, 111, 175, 179, 217,
]);

/// 1-byte discriminator for the Counter account type.
const COUNTER_DISC: u8 = 1;

/// Total on-chain account space: 1-byte disc + u64 value + u8 bump + 7-byte pad.
const COUNTER_SPACE: usize = 1 + 8 + 1 + 7;

#[cfg(not(feature = "no-entrypoint"))]
program_entrypoint!(process_instruction);
no_allocator!();

pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    _instruction_data: &[u8],
) -> ProgramResult {
    // Accounts: [payer (signer, writable), counter (PDA, writable), system_program]
    let payer = unsafe { accounts.get_unchecked(0) };
    let counter = unsafe { accounts.get_unchecked(1) };

    // Derive the counter PDA on-chain — same cost as the framework init macros.
    // The System program's CreateAccount CPI verifies the derived address
    // matches the `to` account, so we don't need an explicit equality check.
    let (_pda, bump) = find_program_address(&[b"counter"], program_id);

    let bump_slice = [bump];
    create_account_signed(
        payer,
        counter,
        COUNTER_SPACE,
        program_id,
        &[b"counter", &bump_slice],
    )?;

    // Write discriminator + initial value + bump via the raw data pointer.
    // Layout: [disc: u8][value: u64 LE][bump: u8][_pad: [u8; 7]]
    unsafe {
        let mut counter_mut = *counter;
        let data = counter_mut.borrow_unchecked_mut();
        *data.get_unchecked_mut(0) = COUNTER_DISC;
        data.get_unchecked_mut(1..9).copy_from_slice(&42u64.to_le_bytes());
        *data.get_unchecked_mut(9) = bump;
    }

    Ok(())
}
