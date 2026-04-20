//! Miri witnesses for the thin wrapper account types.
//!
//! `SystemAccount`, `UncheckedAccount`, `Program<T>`, `Signer` — all
//! `pub struct { view: AccountView }` wrappers with minimal logic.
//! These tests confirm the owner/signer/executable checks in each
//! `load`/`load_mut` behave correctly and don't introduce UB.
//!
//! Run: `cargo +nightly miri test -p anchor-lang-v2 --test miri_wrapper_accounts`

mod common;
use common::AccountBuffer;

use anchor_lang_v2::{
    accounts::{SystemAccount, UncheckedAccount},
    programs::{System, Token},
    prelude::{Program, Signer},
    AnchorAccount,
};
use pinocchio::address::Address;

const PROGRAM_ID: [u8; 32] = [0x42; 32];

// -- SystemAccount ---------------------------------------------------

#[test]
fn system_account_loads_for_system_owned() {
    let mut buf = AccountBuffer::<256>::new();
    buf.init(
        [0x11; 32],
        /*owner*/ [0; 32], // System's ID is all-zero.
        0,
        false,
        true,
        false,
    );
    let view = unsafe { buf.view() };
    let program_id = Address::new_from_array(PROGRAM_ID);
    let acct = SystemAccount::load(view, &program_id).unwrap();
    assert_eq!(acct.address().to_bytes(), [0x11; 32]);
}

#[test]
fn system_account_rejects_non_system_owner() {
    let mut buf = AccountBuffer::<256>::new();
    buf.init([0x11; 32], PROGRAM_ID, 0, false, true, false);
    let view = unsafe { buf.view() };
    let program_id = Address::new_from_array(PROGRAM_ID);
    assert!(SystemAccount::load(view, &program_id).is_err());
}

// -- UncheckedAccount (always loads regardless of owner) -------------

#[test]
fn unchecked_account_loads_for_any_owner() {
    for owner in [[0u8; 32], PROGRAM_ID, [0xFFu8; 32], [0x42u8; 32]] {
        let mut buf = AccountBuffer::<256>::new();
        buf.init([0x22; 32], owner, 0, false, true, false);
        let view = unsafe { buf.view() };
        let program_id = Address::new_from_array(PROGRAM_ID);
        assert!(
            UncheckedAccount::load(view, &program_id).is_ok(),
            "UncheckedAccount must accept any owner: {:?}",
            owner
        );
    }
}

// -- Program<T> ------------------------------------------------------

#[test]
fn program_of_system_loads_when_address_matches_and_executable() {
    let mut buf = AccountBuffer::<256>::new();
    buf.init(
        /*address = System::id()*/ [0; 32],
        [0; 32],
        0,
        false,
        false,
        /*executable*/ true,
    );
    let view = unsafe { buf.view() };
    let program_id = Address::new_from_array(PROGRAM_ID);
    assert!(Program::<System>::load(view, &program_id).is_ok());
}

#[test]
fn program_of_token_rejects_wrong_address() {
    // Buffer claims to be Token, but the address is actually System's.
    let mut buf = AccountBuffer::<256>::new();
    buf.init(
        /*address*/ [0; 32], // System, not Token
        [0; 32],
        0,
        false,
        false,
        true,
    );
    let view = unsafe { buf.view() };
    let program_id = Address::new_from_array(PROGRAM_ID);
    assert!(Program::<Token>::load(view, &program_id).is_err());
}

#[test]
#[cfg(feature = "guardrails")]
fn program_rejects_non_executable_account() {
    // Address matches System, but executable flag is false.
    let mut buf = AccountBuffer::<256>::new();
    buf.init([0; 32], [0; 32], 0, false, false, /*executable*/ false);
    let view = unsafe { buf.view() };
    let program_id = Address::new_from_array(PROGRAM_ID);
    // Under guardrails, Program<T> rejects non-executable.
    assert!(Program::<System>::load(view, &program_id).is_err());
}

// -- Signer ----------------------------------------------------------

#[test]
fn signer_loads_when_is_signer_set() {
    let mut buf = AccountBuffer::<256>::new();
    buf.init([0x33; 32], [0; 32], 0, /*signer*/ true, true, false);
    let view = unsafe { buf.view() };
    let program_id = Address::new_from_array(PROGRAM_ID);
    let signer = Signer::load(view, &program_id).unwrap();
    assert_eq!(signer.address().to_bytes(), [0x33; 32]);
}

#[test]
fn signer_rejects_non_signer() {
    let mut buf = AccountBuffer::<256>::new();
    buf.init([0x33; 32], [0; 32], 0, /*signer*/ false, true, false);
    let view = unsafe { buf.view() };
    let program_id = Address::new_from_array(PROGRAM_ID);
    assert!(Signer::load(view, &program_id).is_err());
}

// -- Aliasing witnesses: multiple wrappers can co-exist if non-conflicting --
//
// Under `AccountView: Copy`, constructing a SystemAccount and then a
// separate UncheckedAccount over the same buffer should work and not
// alias-violate Tree Borrows. This mirrors what happens in a derived
// `#[derive(Accounts)]` struct that holds multiple wrapper fields over
// distinct accounts.

#[test]
fn distinct_wrapper_types_on_distinct_buffers() {
    let mut buf1 = AccountBuffer::<256>::new();
    let mut buf2 = AccountBuffer::<256>::new();
    buf1.init([0x01; 32], [0; 32], 0, false, true, false);
    buf2.init([0x02; 32], PROGRAM_ID, 0, false, true, false);

    let view1 = unsafe { buf1.view() };
    let view2 = unsafe { buf2.view() };
    let program_id = Address::new_from_array(PROGRAM_ID);

    let sys = SystemAccount::load(view1, &program_id).unwrap();
    let unchecked = UncheckedAccount::load(view2, &program_id).unwrap();

    assert_ne!(sys.address().to_bytes(), unchecked.address().to_bytes());
}
