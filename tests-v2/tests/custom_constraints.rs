//! Integration tests for user-defined `AccountConstraint` impls.
//!
//! The `custom-constraints` test program defines four constraint markers
//! in `counter_ns`, each overriding exactly one of the four trait methods
//! (`init`, `check`, `update`, `exit`). These tests drive each handler
//! and inspect the persisted `Counter.value` to prove that the derive
//! routed each call to the correct method at the correct lifecycle
//! phase.

use {
    anchor_lang_v2::{solana_program::instruction::AccountMeta, InstructionData},
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    tests_v2::{build_program, keypair_for, send_instruction},
};

fn program_id() -> Pubkey {
    "CC111111111111111111111111111111111111111111"
        .parse()
        .unwrap()
}

fn counter_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"counter"], &program_id()).0
}

fn setup() -> (LiteSVM, Keypair) {
    let test_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let deploy_dir = test_dir.join("target/deploy");
    build_program(
        test_dir.join("programs/custom-constraints").to_str().unwrap(),
        deploy_dir.to_str().unwrap(),
    );

    let mut svm = LiteSVM::new();
    svm.add_program_from_file(program_id(), &deploy_dir.join("custom_constraints.so"))
        .expect("load custom-constraints program");

    let payer = keypair_for("custom-constraints-payer");
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

/// Parse the Borsh-encoded `Counter.value` from an account. Layout:
/// `[disc: 8][value: 8 LE]`.
fn read_counter_value(svm: &LiteSVM, pda: &Pubkey) -> u64 {
    let account = svm.get_account(pda).expect("counter account exists");
    assert!(account.data.len() >= 16, "counter data too short");
    u64::from_le_bytes(account.data[8..16].try_into().unwrap())
}

fn init_counter(svm: &mut LiteSVM, payer: &Keypair) {
    let data = custom_constraints::instruction::HandleInit {}.data();
    let metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(counter_pda(), false),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
    ];
    send_instruction(svm, program_id(), data, metas, payer, &[])
        .expect("handle_init should succeed");
}

// ---- `init` method ---------------------------------------------------------

#[test]
fn init_hook_stamps_value_on_creation() {
    let (mut svm, payer) = setup();
    init_counter(&mut svm, &payer);

    // `InitValueConstraint::init` wrote 5 after BorshAccount created the
    // zero-filled account. A bare init without the constraint would leave
    // `value == 0`.
    assert_eq!(read_counter_value(&svm, &counter_pda()), 5);
}

// ---- `check` method --------------------------------------------------------

#[test]
fn check_hook_rejects_below_minimum() {
    let (mut svm, payer) = setup();
    init_counter(&mut svm, &payer);
    // After init, value == 5. `handle_check` asserts value >= 10 → fail.

    let data = custom_constraints::instruction::HandleCheck {}.data();
    let metas = vec![AccountMeta::new_readonly(counter_pda(), false)];
    let result = send_instruction(&mut svm, program_id(), data, metas, &payer, &[]);
    assert!(
        result.is_err(),
        "MinValueConstraint::check should reject value 5 against minimum 10",
    );
}

#[test]
fn check_hook_accepts_at_or_above_minimum() {
    let (mut svm, payer) = setup();
    init_counter(&mut svm, &payer);
    // Bump the counter above the minimum via the update hook first.
    let update_data = custom_constraints::instruction::HandleUpdate {}.data();
    let update_metas = vec![AccountMeta::new(counter_pda(), false)];
    send_instruction(&mut svm, program_id(), update_data, update_metas, &payer, &[])
        .expect("handle_update should succeed");
    assert_eq!(read_counter_value(&svm, &counter_pda()), 42);

    // Now check: 42 >= 10 → should pass.
    let data = custom_constraints::instruction::HandleCheck {}.data();
    let metas = vec![AccountMeta::new_readonly(counter_pda(), false)];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("MinValueConstraint::check should accept value 42 against minimum 10");
}

// ---- `update` method -------------------------------------------------------

#[test]
fn update_hook_writes_new_value() {
    let (mut svm, payer) = setup();
    init_counter(&mut svm, &payer);
    assert_eq!(read_counter_value(&svm, &counter_pda()), 5);

    let data = custom_constraints::instruction::HandleUpdate {}.data();
    let metas = vec![AccountMeta::new(counter_pda(), false)];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("handle_update should succeed");

    // `SetValueConstraint::update` overwrote the value.
    assert_eq!(read_counter_value(&svm, &counter_pda()), 42);
}

// ---- `exit` method ---------------------------------------------------------

#[test]
fn exit_hook_mutates_on_successful_instruction() {
    let (mut svm, payer) = setup();
    init_counter(&mut svm, &payer);
    assert_eq!(read_counter_value(&svm, &counter_pda()), 5);

    let data = custom_constraints::instruction::HandleExitBump {}.data();
    let metas = vec![AccountMeta::new(counter_pda(), false)];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("handle_exit_bump should succeed");

    // `BumpOnExitConstraint::exit` added 1 during exit_accounts().
    assert_eq!(read_counter_value(&svm, &counter_pda()), 6);

    // Run it a second time to confirm exit fires on every successful
    // call. LiteSVM re-uses the same blockhash between calls, which
    // would trigger `AlreadyProcessed` on an identical tx — expire
    // before the second send.
    svm.expire_blockhash();
    let data = custom_constraints::instruction::HandleExitBump {}.data();
    let metas = vec![AccountMeta::new(counter_pda(), false)];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("handle_exit_bump should succeed again");
    assert_eq!(read_counter_value(&svm, &counter_pda()), 7);
}

// ---- `init_if_needed` (both `init` and `check` on the create branch) ------

#[test]
fn init_if_needed_create_branch_runs_init_and_check() {
    let (mut svm, payer) = setup();
    // Counter doesn't exist yet. init_if_needed should create it, then
    // `InitValueConstraint::init` stamps value=5, then
    // `MinValueConstraint::check` asserts 5 >= 1 (passes).

    let data = custom_constraints::instruction::HandleInitIfNeeded {}.data();
    let metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(counter_pda(), false),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
    ];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("handle_init_if_needed create branch should succeed");

    assert_eq!(read_counter_value(&svm, &counter_pda()), 5);
}

#[test]
fn init_if_needed_exist_branch_runs_check() {
    let (mut svm, payer) = setup();
    // Pre-populate the counter with a value that satisfies the check.
    init_counter(&mut svm, &payer);
    // Bump above the check's minimum so the exist branch can prove check
    // fires against the LIVE value, not against the init-time default.
    let update_data = custom_constraints::instruction::HandleUpdate {}.data();
    let update_metas = vec![AccountMeta::new(counter_pda(), false)];
    send_instruction(&mut svm, program_id(), update_data, update_metas, &payer, &[])
        .expect("handle_update");
    assert_eq!(read_counter_value(&svm, &counter_pda()), 42);

    // Now init_if_needed should take the exist branch: value is 42,
    // the check min=1 passes, init does NOT fire (value stays 42).
    let data = custom_constraints::instruction::HandleInitIfNeeded {}.data();
    let metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(counter_pda(), false),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
    ];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("handle_init_if_needed exist branch should succeed");

    assert_eq!(
        read_counter_value(&svm, &counter_pda()),
        42,
        "exist branch must not re-run init (would have overwritten 42 → 5)",
    );
}
