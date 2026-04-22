//! Negative-path tests for the view-wrapper account types.
//!
//! Each wrapper's `load` / `load_mut` runs a small number of checks
//! (`is_signer`, `is_writable`, owner match, address match) before
//! returning a typed handle. When the check fails the wrapper must
//! surface a precise `ProgramError`, not silently accept the account.
//!
//! These tests pin those rejection paths because they're the security
//! boundary — the derive-level constraint layer runs *after* the
//! wrapper's own gate, so a wrapper false-accept is an unrecoverable
//! auth bypass (see `program.rs` and `sysvar.rs` docs on why
//! `address = X @ MyErr` cannot override these).
//!
//! Run: `cargo test -p anchor-lang-v2 --features testing --test account_wrapper_checks`

use {
    anchor_lang_v2::{
        accounts::{Program, Signer, SystemAccount, Sysvar, UncheckedAccount},
        programs::{System, Token},
        testing::AccountBuffer,
        AnchorAccount, ErrorCode,
    },
    pinocchio::address::Address,
    solana_program_error::ProgramError,
};

const PROGRAM_ID: [u8; 32] = [0x42; 32];
const SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];

fn program_id() -> Address {
    Address::new_from_array(PROGRAM_ID)
}

// The account wrappers don't `#[derive(Debug)]`, so `Result::unwrap_err`
// can't format the `Ok` branch. Local helper extracts the error without
// triggering the `T: Debug` bound.
fn expect_err<T>(r: Result<T, ProgramError>) -> ProgramError {
    match r {
        Ok(_) => panic!("expected Err, got Ok"),
        Err(e) => e,
    }
}

// -- Signer -------------------------------------------------------------

#[test]
fn signer_load_rejects_non_signer() {
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0x01; 32], SYSTEM_PROGRAM_ID, 0, /*signer*/ false, false, false);
    let view = unsafe { buf.view() };
    let err = expect_err(Signer::load(view, &program_id()));
    assert_eq!(err, ProgramError::MissingRequiredSignature);
}

#[test]
fn signer_load_accepts_signer() {
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0x01; 32], SYSTEM_PROGRAM_ID, 0, /*signer*/ true, false, false);
    let view = unsafe { buf.view() };
    let signer = Signer::load(view, &program_id()).unwrap();
    assert_eq!(signer.address().to_bytes(), [0x01; 32]);
}

#[test]
fn signer_load_mut_rejects_non_signer_non_writable() {
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0x01; 32], SYSTEM_PROGRAM_ID, 0, false, false, false);
    let view = unsafe { buf.view() };
    let err = expect_err(unsafe { Signer::load_mut(view, &program_id()) });
    // Fused check: either flag missing maps to ConstraintSigner.
    assert_eq!(err, ErrorCode::ConstraintSigner.into());
}

#[test]
fn signer_load_mut_rejects_signer_without_writable() {
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0x01; 32], SYSTEM_PROGRAM_ID, 0, /*signer*/ true, /*writable*/ false, false);
    let view = unsafe { buf.view() };
    let err = expect_err(unsafe { Signer::load_mut(view, &program_id()) });
    assert_eq!(err, ErrorCode::ConstraintSigner.into());
}

#[test]
fn signer_load_mut_rejects_writable_without_signer() {
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0x01; 32], SYSTEM_PROGRAM_ID, 0, /*signer*/ false, /*writable*/ true, false);
    let view = unsafe { buf.view() };
    let err = expect_err(unsafe { Signer::load_mut(view, &program_id()) });
    assert_eq!(err, ErrorCode::ConstraintSigner.into());
}

#[test]
fn signer_load_mut_accepts_signer_and_writable() {
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0x01; 32], SYSTEM_PROGRAM_ID, 0, /*signer*/ true, /*writable*/ true, false);
    let view = unsafe { buf.view() };
    let signer = unsafe { Signer::load_mut(view, &program_id()) }.unwrap();
    assert_eq!(signer.address().to_bytes(), [0x01; 32]);
}

// -- SystemAccount ------------------------------------------------------

#[test]
fn system_account_load_rejects_non_system_owner() {
    let mut buf = AccountBuffer::<128>::new();
    // Owner = [0x42; 32] (program_id), not the all-zero System program id.
    buf.init([0x01; 32], PROGRAM_ID, 0, false, false, false);
    let view = unsafe { buf.view() };
    let err = expect_err(SystemAccount::load(view, &program_id()));
    assert_eq!(err, ProgramError::IllegalOwner);
}

#[test]
fn system_account_load_accepts_system_owner() {
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0x01; 32], SYSTEM_PROGRAM_ID, 0, false, false, false);
    let view = unsafe { buf.view() };
    let sa = SystemAccount::load(view, &program_id()).unwrap();
    assert_eq!(sa.address().to_bytes(), [0x01; 32]);
}

#[test]
fn system_account_default_load_mut_rejects_non_writable() {
    // SystemAccount doesn't override `load_mut`, so the default impl runs
    // an `is_writable` check first — a non-writable account must surface
    // `ConstraintMut`, not `IllegalOwner`, even though the owner check
    // would also fail.
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0x01; 32], PROGRAM_ID, 0, false, /*writable*/ false, false);
    let view = unsafe { buf.view() };
    let err = expect_err(unsafe { SystemAccount::load_mut(view, &program_id()) });
    assert_eq!(err, ErrorCode::ConstraintMut.into());
}

#[test]
fn system_account_default_load_mut_rejects_writable_wrong_owner() {
    // Writable passes, then the owner check fires.
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0x01; 32], PROGRAM_ID, 0, false, /*writable*/ true, false);
    let view = unsafe { buf.view() };
    let err = expect_err(unsafe { SystemAccount::load_mut(view, &program_id()) });
    assert_eq!(err, ProgramError::IllegalOwner);
}

// -- UncheckedAccount ---------------------------------------------------

#[test]
fn unchecked_account_load_accepts_anything() {
    // Whatever the flags / owner, `UncheckedAccount::load` must succeed:
    // it's the escape hatch for programs that want to run validation
    // themselves in a derive-level `address = X @ MyErr` constraint.
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0xAB; 32], [0x99; 32], 0, false, false, false);
    let view = unsafe { buf.view() };
    let ua = UncheckedAccount::load(view, &program_id()).unwrap();
    assert_eq!(ua.address().to_bytes(), [0xAB; 32]);
}

#[test]
fn unchecked_account_default_load_mut_rejects_non_writable() {
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0xAB; 32], [0x99; 32], 0, false, /*writable*/ false, false);
    let view = unsafe { buf.view() };
    let err = expect_err(unsafe { UncheckedAccount::load_mut(view, &program_id()) });
    assert_eq!(err, ErrorCode::ConstraintMut.into());
}

#[test]
fn unchecked_account_load_mut_accepts_writable() {
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0xAB; 32], [0x99; 32], 0, false, /*writable*/ true, false);
    let view = unsafe { buf.view() };
    let ua = unsafe { UncheckedAccount::load_mut(view, &program_id()) }.unwrap();
    assert_eq!(ua.address().to_bytes(), [0xAB; 32]);
}

// -- Program<T> ---------------------------------------------------------

#[test]
fn program_load_rejects_wrong_address() {
    let mut buf = AccountBuffer::<128>::new();
    // Address = [0x01; 32], expecting System (all-zero).
    buf.init([0x01; 32], [0u8; 32], 0, false, false, /*executable*/ true);
    let view = unsafe { buf.view() };
    let err = expect_err(Program::<System>::load(view, &program_id()));
    assert_eq!(err, ProgramError::IncorrectProgramId);
}

#[test]
fn program_load_accepts_matching_system_address() {
    let mut buf = AccountBuffer::<128>::new();
    // System program address is all-zero.
    buf.init([0u8; 32], [0u8; 32], 0, false, false, /*executable*/ true);
    let view = unsafe { buf.view() };
    let p = Program::<System>::load(view, &program_id()).unwrap();
    assert_eq!(p.address().to_bytes(), [0u8; 32]);
}

#[cfg(feature = "guardrails")]
#[test]
fn program_load_rejects_non_executable_under_guardrails() {
    let mut buf = AccountBuffer::<128>::new();
    // Correct address but not executable.
    buf.init([0u8; 32], [0u8; 32], 0, false, false, /*executable*/ false);
    let view = unsafe { buf.view() };
    let err = expect_err(Program::<System>::load(view, &program_id()));
    assert_eq!(err, ProgramError::InvalidAccountData);
}

#[test]
fn program_load_token_wrong_address_rejects() {
    // Arbitrary non-Token address — must reject on the address compare.
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0x01; 32], [0u8; 32], 0, false, false, /*executable*/ true);
    let view = unsafe { buf.view() };
    let err = expect_err(Program::<Token>::load(view, &program_id()));
    assert_eq!(err, ProgramError::IncorrectProgramId);
}

// -- Sysvar<T> ----------------------------------------------------------

#[test]
fn sysvar_load_rejects_wrong_address() {
    // Passing a non-Clock address for `Sysvar<Clock>` must reject before
    // any syscall runs — see `sysvar.rs`'s `InvalidArgument` path.
    let mut buf = AccountBuffer::<128>::new();
    buf.init([0x01; 32], [0u8; 32], 0, false, false, false);
    let view = unsafe { buf.view() };
    let err = expect_err(Sysvar::<pinocchio::sysvars::clock::Clock>::load(view, &program_id()));
    assert_eq!(err, ProgramError::InvalidArgument);
}
