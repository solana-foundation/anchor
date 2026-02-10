/// Security tests for init_if_needed vulnerability fixes.
///
/// V-1: Incomplete field validation for token accounts in init_if_needed.
///   When init_if_needed encounters an already-existing token account, it previously
///   only validated mint, owner, and token_program. delegate and close_authority were
///   not checked, allowing an attacker to pre-create an account with malicious
///   authorities that survive the init_if_needed validation.
///
/// V-2: init_if_needed accounts excluded from duplicate mutable account check.
///   The duplicate mutable account filter used `f.constraints.init.is_none()` which
///   excluded ALL init accounts. This is correct for pure init (create_account would
///   fail on duplicates), but init_if_needed accepts existing accounts and must be
///   included in duplicate detection.

use anchor_lang::error::{Error, ErrorCode, ErrorOrigin};

// ---------------------------------------------------------------------------
// V-1: Token delegate / close_authority field validation
// ---------------------------------------------------------------------------

/// Verify the new error codes exist at the correct offsets (4200-4202).
/// These must be appended AFTER existing codes to avoid shifting the ABI.
#[test]
fn v1_error_codes_at_correct_offset() {
    let delegate: u32 = ErrorCode::ConstraintTokenDelegate.into();
    let close_auth: u32 = ErrorCode::ConstraintTokenCloseAuthority.into();
    let state: u32 = ErrorCode::ConstraintTokenAccountState.into();

    assert_eq!(delegate, 4200);
    assert_eq!(close_auth, 4201);
    assert_eq!(state, 4202);

    // Verify they are sequential
    assert_eq!(close_auth, delegate + 1);
    assert_eq!(state, delegate + 2);
}

/// Verify that existing error codes are not displaced by the new additions.
/// Any shift would break the ABI for all deployed programs.
#[test]
fn v1_existing_error_codes_not_displaced() {
    // Constraint codes
    assert_eq!(u32::from(ErrorCode::ConstraintMut), 2000);
    assert_eq!(u32::from(ErrorCode::ConstraintTokenMint), 2014);
    assert_eq!(u32::from(ErrorCode::ConstraintTokenOwner), 2015);
    assert_eq!(u32::from(ErrorCode::ConstraintTokenTokenProgram), 2021);
    assert_eq!(u32::from(ErrorCode::ConstraintDuplicateMutableAccount), 2040);

    // Account codes
    assert_eq!(u32::from(ErrorCode::AccountDiscriminatorNotFound), 3001);
    assert_eq!(u32::from(ErrorCode::AccountNotAssociatedTokenAccount), 3014);

    // Misc codes
    assert_eq!(u32::from(ErrorCode::DeclaredProgramIdMismatch), 4100);
    assert_eq!(u32::from(ErrorCode::InvalidNumericConversion), 4102);

    // Deprecated sentinel
    assert_eq!(u32::from(ErrorCode::Deprecated), 5000);
}

/// Verify that the error construction chain used by the generated code works
/// correctly: Error::from(ErrorCode) + .with_account_name().
/// This matches the exact pattern in the codegen at constraints.rs:676-681.
#[test]
fn v1_delegate_error_carries_account_name() {
    let err = Error::from(ErrorCode::ConstraintTokenDelegate).with_account_name("my_token");

    match err {
        Error::AnchorError(ae) => {
            assert_eq!(ae.error_code_number, 4200);
            assert_eq!(ae.error_name, "ConstraintTokenDelegate");
            assert!(
                ae.error_msg
                    .contains("delegate must not be set"),
                "error message should describe the delegate constraint: {}",
                ae.error_msg
            );
            match &ae.error_origin {
                Some(ErrorOrigin::AccountName(name)) => {
                    assert_eq!(name, "my_token");
                }
                other => panic!(
                    "expected ErrorOrigin::AccountName, got {:?}",
                    other
                ),
            }
        }
        Error::ProgramError(_) => panic!("expected AnchorError, got ProgramError"),
    }
}

/// Same test for close_authority error — verifies the generated code path
/// at constraints.rs:679-681.
#[test]
fn v1_close_authority_error_carries_account_name() {
    let err =
        Error::from(ErrorCode::ConstraintTokenCloseAuthority).with_account_name("victim_ata");

    match err {
        Error::AnchorError(ae) => {
            assert_eq!(ae.error_code_number, 4201);
            assert_eq!(ae.error_name, "ConstraintTokenCloseAuthority");
            assert!(
                ae.error_msg.contains("close authority must not be set"),
                "error message should describe the close authority constraint: {}",
                ae.error_msg
            );
            match &ae.error_origin {
                Some(ErrorOrigin::AccountName(name)) => {
                    assert_eq!(name, "victim_ata");
                }
                other => panic!(
                    "expected ErrorOrigin::AccountName, got {:?}",
                    other
                ),
            }
        }
        Error::ProgramError(_) => panic!("expected AnchorError, got ProgramError"),
    }
}

/// Verify that the new V-1 error codes are distinct from the existing token
/// constraint errors they're related to. An incorrect offset would cause
/// collisions with existing codes.
#[test]
fn v1_new_codes_distinct_from_existing_token_constraints() {
    let existing_token_errors: Vec<u32> = vec![
        ErrorCode::ConstraintTokenMint.into(),
        ErrorCode::ConstraintTokenOwner.into(),
        ErrorCode::ConstraintTokenTokenProgram.into(),
        ErrorCode::ConstraintAssociatedInit.into(),
        ErrorCode::ConstraintAssociatedTokenTokenProgram.into(),
    ];

    let new_errors: Vec<u32> = vec![
        ErrorCode::ConstraintTokenDelegate.into(),
        ErrorCode::ConstraintTokenCloseAuthority.into(),
        ErrorCode::ConstraintTokenAccountState.into(),
    ];

    for new_code in &new_errors {
        assert!(
            !existing_token_errors.contains(new_code),
            "new error code {} collides with an existing token constraint error",
            new_code
        );
    }
}

/// Verify ErrorCode::name() returns the correct variant name strings.
/// The generated code logs these names in error messages.
#[test]
fn v1_error_code_names_are_correct() {
    assert_eq!(
        ErrorCode::ConstraintTokenDelegate.name(),
        "ConstraintTokenDelegate"
    );
    assert_eq!(
        ErrorCode::ConstraintTokenCloseAuthority.name(),
        "ConstraintTokenCloseAuthority"
    );
    assert_eq!(
        ErrorCode::ConstraintTokenAccountState.name(),
        "ConstraintTokenAccountState"
    );
}

// ---------------------------------------------------------------------------
// V-2: Duplicate mutable account check for init_if_needed
// ---------------------------------------------------------------------------

/// Verify the ConstraintDuplicateMutableAccount error code and message are
/// correct. This is the error that would be returned when the same account
/// is passed in two mutable positions.
#[test]
fn v2_duplicate_mutable_error_code_exists() {
    let code: u32 = ErrorCode::ConstraintDuplicateMutableAccount.into();
    assert_eq!(code, 2040);
}

/// Verify the full error chain for duplicate mutable accounts includes
/// the account name, matching the generated code pattern.
#[test]
fn v2_duplicate_mutable_error_carries_account_name() {
    let err = Error::from(ErrorCode::ConstraintDuplicateMutableAccount)
        .with_account_name("data_account");

    match err {
        Error::AnchorError(ae) => {
            assert_eq!(ae.error_code_number, 2040);
            assert!(
                ae.error_msg.contains("duplicate"),
                "error message should mention duplicate: {}",
                ae.error_msg
            );
            match &ae.error_origin {
                Some(ErrorOrigin::AccountName(name)) => {
                    assert_eq!(name, "data_account");
                }
                other => panic!(
                    "expected ErrorOrigin::AccountName, got {:?}",
                    other
                ),
            }
        }
        Error::ProgramError(_) => panic!("expected AnchorError, got ProgramError"),
    }
}

/// Demonstrate the V-2 filter logic behavior.
///
/// The fix changes the duplicate mutable check filter from:
///   `f.constraints.init.is_none()`          (excludes ALL init accounts)
/// to:
///   `!matches!(&f.constraints.init, Some(init) if !init.if_needed)`
///   (excludes only pure init, includes init_if_needed)
///
/// This test validates the boolean logic of the matches! expression
/// against all three cases: no init, pure init, and init_if_needed.
#[test]
fn v2_filter_logic_includes_init_if_needed() {
    // Simulate the ConstraintInit struct
    struct MockInit {
        if_needed: bool,
    }

    // Case 1: No init constraint (init is None)
    // Old filter: None.is_none() = true  → included ✓
    // New filter: !matches!(None, Some(init) if !init.if_needed) = !false = true → included ✓
    let no_init: Option<MockInit> = None;
    let old_includes_no_init = no_init.is_none();
    let new_includes_no_init = !matches!(&no_init, Some(init) if !init.if_needed);
    assert!(old_includes_no_init, "old filter should include non-init accounts");
    assert!(
        new_includes_no_init,
        "new filter should include non-init accounts"
    );

    // Case 2: Pure init (init_if_needed = false)
    // Old filter: Some(_).is_none() = false → excluded ✓
    // New filter: !matches!(Some(init), ... if !init.if_needed)
    //           = !matches!(Some(MockInit{false}), Some(init) if !false)
    //           = !matches!(Some(..), Some(init) if true)
    //           = !true = false → excluded ✓
    let pure_init: Option<MockInit> = Some(MockInit { if_needed: false });
    let old_includes_pure_init = pure_init.is_none();
    let new_includes_pure_init = !matches!(&pure_init, Some(init) if !init.if_needed);
    assert!(
        !old_includes_pure_init,
        "old filter should exclude pure init"
    );
    assert!(
        !new_includes_pure_init,
        "new filter should exclude pure init"
    );

    // Case 3: init_if_needed (if_needed = true) — THE CRITICAL CASE
    // Old filter: Some(_).is_none() = false → excluded ✗ (BUG)
    // New filter: !matches!(Some(init), ... if !init.if_needed)
    //           = !matches!(Some(MockInit{true}), Some(init) if !true)
    //           = !matches!(Some(..), Some(init) if false)
    //           = !false = true → included ✓ (FIXED)
    let init_if_needed: Option<MockInit> = Some(MockInit { if_needed: true });
    let old_includes_init_if_needed = init_if_needed.is_none();
    let new_includes_init_if_needed =
        !matches!(&init_if_needed, Some(init) if !init.if_needed);

    assert!(
        !old_includes_init_if_needed,
        "old filter INCORRECTLY excluded init_if_needed from duplicate check"
    );
    assert!(
        new_includes_init_if_needed,
        "new filter correctly includes init_if_needed in duplicate check"
    );

    // Verify the behavioral difference: old and new disagree ONLY for init_if_needed
    assert_eq!(
        old_includes_no_init, new_includes_no_init,
        "no-init case: both filters agree"
    );
    assert_eq!(
        old_includes_pure_init, new_includes_pure_init,
        "pure-init case: both filters agree"
    );
    assert_ne!(
        old_includes_init_if_needed, new_includes_init_if_needed,
        "init_if_needed case: filters MUST disagree — this is the bug fix"
    );
}
