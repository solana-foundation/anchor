//! Integration tests for account-wrapper coverage — Sysvar, Box<Account>,
//! SystemAccount, UncheckedAccount.

use {
    anchor_lang_v2::solana_program::instruction::AccountMeta,
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    tests_v2::{build_program, keypair_for, send_instruction},
};

fn program_id() -> Pubkey {
    "Acc1111111111111111111111111111111111111111".parse().unwrap()
}

fn clock_sysvar_id() -> Pubkey {
    "SysvarC1ock11111111111111111111111111111111".parse().unwrap()
}

fn rent_sysvar_id() -> Pubkey {
    "SysvarRent111111111111111111111111111111111".parse().unwrap()
}

fn counter_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"counter"], &program_id()).0
}

fn setup() -> (LiteSVM, Keypair) {
    let test_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let deploy_dir = test_dir.join("target/deploy");
    build_program(
        test_dir.join("programs/accounts").to_str().unwrap(),
        deploy_dir.to_str().unwrap(),
    );

    let mut svm = LiteSVM::new();
    svm.add_program_from_file(program_id(), deploy_dir.join("accounts_test.so"))
        .expect("load accounts_test program");
    let payer = keypair_for("accounts-payer");
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

fn do_initialize(svm: &mut LiteSVM, payer: &Keypair) -> Pubkey {
    let counter = counter_pda();
    let metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(counter, false),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
    ];
    send_instruction(svm, program_id(), vec![0], metas, payer, &[])
        .expect("initialize should succeed");
    counter
}

#[test]
fn initialize_creates_counter_with_value_one() {
    let (mut svm, payer) = setup();
    let counter = do_initialize(&mut svm, &payer);
    let account = svm.get_account(&counter).expect("counter exists");
    // 8-byte disc + u64 value. disc prefix = 8 bytes.
    let value = u64::from_le_bytes(account.data[8..16].try_into().unwrap());
    assert_eq!(value, 1);
}

#[test]
fn bump_boxed_mutates_through_box_deref() {
    let (mut svm, payer) = setup();
    let counter = do_initialize(&mut svm, &payer);

    // bump_boxed (discrim = 1) — value = 1 + 1 = 2
    let metas = vec![AccountMeta::new(counter, false)];
    send_instruction(&mut svm, program_id(), vec![1], metas, &payer, &[])
        .expect("bump_boxed should succeed");

    let account = svm.get_account(&counter).expect("counter exists");
    let value = u64::from_le_bytes(account.data[8..16].try_into().unwrap());
    assert_eq!(value, 2);
}

#[test]
fn read_clock_succeeds_with_clock_sysvar() {
    let (mut svm, payer) = setup();
    let metas = vec![AccountMeta::new_readonly(clock_sysvar_id(), false)];
    send_instruction(&mut svm, program_id(), vec![2], metas, &payer, &[])
        .expect("read_clock should succeed");
}

#[test]
fn read_clock_rejects_wrong_sysvar() {
    let (mut svm, payer) = setup();
    // Passing rent instead of clock trips `T::SYSVAR_ID` compare in
    // `Sysvar<Clock>::load`.
    let metas = vec![AccountMeta::new_readonly(rent_sysvar_id(), false)];
    let result = send_instruction(&mut svm, program_id(), vec![2], metas, &payer, &[]);
    assert!(result.is_err(), "wrong sysvar should be rejected");
}

#[test]
fn read_rent_succeeds_with_rent_sysvar() {
    let (mut svm, payer) = setup();
    let metas = vec![AccountMeta::new_readonly(rent_sysvar_id(), false)];
    send_instruction(&mut svm, program_id(), vec![3], metas, &payer, &[])
        .expect("read_rent should succeed");
}

#[test]
fn check_system_accepts_system_owned_account() {
    let (mut svm, payer) = setup();
    // `payer` was funded via airdrop, so it's owned by the System program.
    let wallet = keypair_for("wallet");
    svm.airdrop(&wallet.pubkey(), 1_000_000).unwrap();

    let metas = vec![AccountMeta::new_readonly(wallet.pubkey(), false)];
    send_instruction(&mut svm, program_id(), vec![4], metas, &payer, &[])
        .expect("check_system should succeed");
}

#[test]
fn check_system_rejects_non_system_owned() {
    let (mut svm, payer) = setup();
    let counter = do_initialize(&mut svm, &payer);

    // `counter` is owned by our program, not the System program.
    let metas = vec![AccountMeta::new_readonly(counter, false)];
    let result = send_instruction(&mut svm, program_id(), vec![4], metas, &payer, &[]);
    assert!(result.is_err(), "non-system-owned account should be rejected");
}

#[test]
fn touch_unchecked_accepts_arbitrary_account() {
    let (mut svm, payer) = setup();
    // UncheckedAccount does no owner/address validation — any account passes.
    let any = keypair_for("anyone");
    let metas = vec![AccountMeta::new_readonly(any.pubkey(), false)];
    send_instruction(&mut svm, program_id(), vec![5], metas, &payer, &[])
        .expect("touch_unchecked should succeed");
}
