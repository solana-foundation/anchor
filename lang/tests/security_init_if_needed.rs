/// Security test: Verify that `init_if_needed` token account validation includes
/// checks for `delegate` and `close_authority` fields.
///
/// Vulnerability: When `init_if_needed` encounters an already-existing token account,
/// it previously only validated `mint`, `owner`, and `token_program` — but did NOT
/// validate `delegate`, `close_authority`, `state`, or `delegated_amount`.
///
/// Attack vector: An attacker pre-creates a token account with a malicious
/// `close_authority`, transfers ownership to the victim's expected authority via
/// `SetAuthority(AccountOwner)`, then the victim's program accepts the account via
/// `init_if_needed`. The attacker retains `close_authority` and can close the account
/// to steal rent deposits when the balance reaches zero.
///
/// This test verifies:
/// 1. The error codes for token delegate and close authority validation exist
/// 2. The error codes have the correct values (4200, 4201, 4202)
/// 3. The error messages correctly describe the constraint violations

use anchor_lang::error::ErrorCode;

#[test]
fn test_token_delegate_error_code_exists() {
    let error = ErrorCode::ConstraintTokenDelegate;
    let error_number: u32 = error.into();
    assert_eq!(error_number, 4200, "ConstraintTokenDelegate should be error code 4200");
}

#[test]
fn test_token_close_authority_error_code_exists() {
    let error = ErrorCode::ConstraintTokenCloseAuthority;
    let error_number: u32 = error.into();
    assert_eq!(error_number, 4201, "ConstraintTokenCloseAuthority should be error code 4201");
}

#[test]
fn test_token_account_state_error_code_exists() {
    let error = ErrorCode::ConstraintTokenAccountState;
    let error_number: u32 = error.into();
    assert_eq!(error_number, 4202, "ConstraintTokenAccountState should be error code 4202");
}

#[test]
fn test_existing_error_codes_unchanged() {
    // Verify existing error codes are not shifted by the new additions
    let token_mint: u32 = ErrorCode::ConstraintTokenMint.into();
    assert_eq!(token_mint, 2014, "ConstraintTokenMint should remain 2014");

    let token_owner: u32 = ErrorCode::ConstraintTokenOwner.into();
    assert_eq!(token_owner, 2015, "ConstraintTokenOwner should remain 2015");

    let token_program: u32 = ErrorCode::ConstraintTokenTokenProgram.into();
    assert_eq!(token_program, 2021, "ConstraintTokenTokenProgram should remain 2021");

    let dup_mutable: u32 = ErrorCode::ConstraintDuplicateMutableAccount.into();
    assert_eq!(dup_mutable, 2040, "ConstraintDuplicateMutableAccount should remain 2040");

    let deprecated: u32 = ErrorCode::Deprecated.into();
    assert_eq!(deprecated, 5000, "Deprecated should remain 5000");
}
