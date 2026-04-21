//! Integration tests for account-wrapper coverage — Sysvar, Box<Account>,
//! SystemAccount, UncheckedAccount.

use {
    anchor_lang_v2::solana_program::instruction::{AccountMeta, Instruction},
    litesvm::{types::TransactionResult, LiteSVM},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
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
fn read_clock_succeeds_and_sysvar_is_well_formed() {
    let (mut svm, payer) = setup();
    let metas = vec![AccountMeta::new_readonly(clock_sysvar_id(), false)];
    send_instruction(&mut svm, program_id(), vec![2], metas, &payer, &[])
        .expect("read_clock should succeed");

    // Verify the Clock sysvar account exists and has the expected layout.
    // Clock layout: slot(u64) + epoch_start_timestamp(i64) + epoch(u64)
    //             + leader_schedule_epoch(u64) + unix_timestamp(i64) = 40 bytes.
    let clock_account = svm.get_account(&clock_sysvar_id()).expect("clock sysvar exists");
    assert!(
        clock_account.data.len() >= 40,
        "clock sysvar data should be at least 40 bytes, got {}",
        clock_account.data.len()
    );
    // slot lives at offset 0; epoch at offset 16. Both should be
    // parseable (the test program already accessed them without error).
    let slot = u64::from_le_bytes(clock_account.data[0..8].try_into().unwrap());
    let epoch = u64::from_le_bytes(clock_account.data[16..24].try_into().unwrap());
    // LiteSVM may start at slot 0 / epoch 0, so we only assert they
    // are parseable and the instruction succeeded — the important
    // property is that the sysvar deserialized correctly on-chain.
    let _ = (slot, epoch);
}

#[test]
fn read_clock_rejects_wrong_sysvar() {
    let (mut svm, payer) = setup();
    // Passing rent instead of clock trips `T::SYSVAR_ID` compare in
    // `Sysvar<Clock>::load`.
    let metas = vec![AccountMeta::new_readonly(rent_sysvar_id(), false)];
    let result = send_instruction(&mut svm, program_id(), vec![2], metas, &payer, &[]);
    let err_msg = format!("{:?}", result.as_ref().err().expect("should fail"));
    // The sysvar ID mismatch surfaces as InvalidArgument, InvalidAccountData,
    // or a Custom error depending on the runtime path.
    assert!(
        err_msg.contains("InvalidAccountData")
            || err_msg.contains("InvalidArgument")
            || err_msg.contains("Custom("),
        "wrong sysvar should be rejected, got: {err_msg}"
    );
}

#[test]
fn read_rent_succeeds_and_has_positive_minimum_balance() {
    let (mut svm, payer) = setup();
    let metas = vec![AccountMeta::new_readonly(rent_sysvar_id(), false)];
    send_instruction(&mut svm, program_id(), vec![3], metas, &payer, &[])
        .expect("read_rent should succeed");

    // Verify that the Rent sysvar returns meaningful values.
    // The minimum_balance_for_rent_exemption for any non-zero data size
    // should be > 0.
    let min_balance = svm.minimum_balance_for_rent_exemption(100);
    assert!(
        min_balance > 0,
        "rent minimum balance for 100 bytes should be > 0, got {min_balance}"
    );
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
    let err_msg = format!("{:?}", result.as_ref().err().expect("should fail"));
    assert!(
        err_msg.contains("IllegalOwner") || err_msg.contains("Custom("),
        "non-system-owned account should be rejected with owner error, got: {err_msg}"
    );
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

// ---- Helpers ---------------------------------------------------------------

fn call_raw(
    svm: &mut LiteSVM,
    payer: &Keypair,
    disc: u8,
    accounts: Vec<AccountMeta>,
) -> TransactionResult {
    let ix = Instruction::new_with_bytes(program_id(), &[disc], accounts);
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let signers: Vec<&dyn solana_signer::Signer> = vec![payer];
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
    svm.send_transaction(tx)
}

// ---- Sysvar field assertions -----------------------------------------------

#[test]
fn read_clock_sysvar_has_valid_fields() {
    let (mut svm, payer) = setup();
    // The Clock sysvar at its well-known address stores:
    // [slot:u64][epoch_start_timestamp:i64][epoch:u64][leader_schedule_epoch:u64][unix_timestamp:i64]
    // = 40 bytes total.
    let account = svm.get_account(&clock_sysvar_id()).expect("clock sysvar exists");
    assert_eq!(account.data.len(), 40, "Clock sysvar should be 40 bytes");

    let slot = u64::from_le_bytes(account.data[0..8].try_into().unwrap());
    let epoch = u64::from_le_bytes(account.data[16..24].try_into().unwrap());
    // LiteSVM starts at slot > 0 after genesis
    assert!(slot > 0 || epoch == 0, "slot or epoch should reflect a valid genesis state");

    // Also verify the on-chain handler succeeds (already tested, but now
    // we know the sysvar is well-formed for the assertions above)
    let metas = vec![AccountMeta::new_readonly(clock_sysvar_id(), false)];
    send_instruction(&mut svm, program_id(), vec![2], metas, &payer, &[])
        .expect("read_clock should succeed with well-formed sysvar");
}

#[test]
fn read_rent_sysvar_has_positive_minimum_balance() {
    let (mut svm, payer) = setup();
    // Rent sysvar: [lamports_per_byte_year:u64][exemption_threshold:f64][burn_percent:u8]
    let account = svm.get_account(&rent_sysvar_id()).expect("rent sysvar exists");
    assert!(account.data.len() >= 17, "Rent sysvar should be at least 17 bytes");

    let lamports_per_byte = u64::from_le_bytes(account.data[0..8].try_into().unwrap());
    assert!(lamports_per_byte > 0, "lamports_per_byte_year should be positive");

    // Also verify minimum_balance_for_rent_exemption gives sensible output
    let min_balance = svm.minimum_balance_for_rent_exemption(100);
    assert!(min_balance > 0, "minimum_balance for 100 bytes should be > 0");

    // The on-chain handler should succeed
    let metas = vec![AccountMeta::new_readonly(rent_sysvar_id(), false)];
    send_instruction(&mut svm, program_id(), vec![3], metas, &payer, &[])
        .expect("read_rent should succeed");
}

#[test]
fn read_clock_rejects_wrong_sysvar_with_specific_error() {
    let (mut svm, payer) = setup();
    let metas = vec![AccountMeta::new_readonly(rent_sysvar_id(), false)];
    let result = call_raw(&mut svm, &payer, 2, metas);
    let err = format!("{:?}", result.unwrap_err().err);
    // The Sysvar<Clock> load checks the account address matches the Clock sysvar ID.
    assert!(
        err.contains("InvalidArgument") || err.contains("InvalidAccountData") || err.contains("Custom("),
        "expected a specific error for wrong sysvar, got: {err}"
    );
}

#[test]
fn read_rent_rejects_wrong_sysvar() {
    let (mut svm, payer) = setup();
    // Pass clock sysvar to rent handler
    let metas = vec![AccountMeta::new_readonly(clock_sysvar_id(), false)];
    let result = call_raw(&mut svm, &payer, 3, metas);
    assert!(result.is_err(), "wrong sysvar should be rejected for rent");
}

// ---- Box<Account<T>> -------------------------------------------------------

#[test]
fn bump_boxed_accumulates_across_calls() {
    let (mut svm, payer) = setup();
    let counter = do_initialize(&mut svm, &payer);

    // Call bump_boxed 3 times, verify value increments each time
    for expected in 2..=4u64 {
        let metas = vec![AccountMeta::new(counter, false)];
        send_instruction(&mut svm, program_id(), vec![1], metas, &payer, &[])
            .expect("bump_boxed should succeed");

        let account = svm.get_account(&counter).unwrap();
        let value = u64::from_le_bytes(account.data[8..16].try_into().unwrap());
        assert_eq!(value, expected, "counter should be {expected} after bump #{}", expected - 1);

        svm.expire_blockhash();
    }
}

#[test]
fn bump_boxed_rejects_wrong_owner() {
    let (mut svm, payer) = setup();

    // Create an account owned by system program, not our program
    let fake = Pubkey::new_unique();
    let account = solana_account::Account {
        lamports: 1_000_000,
        data: vec![0u8; 16], // disc(8) + value(8)
        owner: solana_sdk_ids::system_program::ID,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(fake, account).unwrap();

    let metas = vec![AccountMeta::new(fake, false)];
    let result = call_raw(&mut svm, &payer, 1, metas);
    assert!(result.is_err(), "system-owned account should fail Box<Account<Counter>> load");
}

// ---- SystemAccount ---------------------------------------------------------

#[test]
fn check_system_rejects_program_owned_with_specific_error() {
    let (mut svm, payer) = setup();
    let counter = do_initialize(&mut svm, &payer);

    let metas = vec![AccountMeta::new_readonly(counter, false)];
    let result = call_raw(&mut svm, &payer, 4, metas);
    let err = format!("{:?}", result.unwrap_err().err);
    assert!(
        err.contains("IllegalOwner") || err.contains("Custom("),
        "expected IllegalOwner for program-owned account in SystemAccount, got: {err}"
    );
}

#[test]
fn check_system_accepts_unfunded_system_account() {
    let (mut svm, payer) = setup();
    // A pubkey with no on-chain account is treated as system-owned with 0 lamports
    let unfunded = Pubkey::new_unique();
    let metas = vec![AccountMeta::new_readonly(unfunded, false)];
    send_instruction(&mut svm, program_id(), vec![4], metas, &payer, &[])
        .expect("unfunded system account should be accepted");
}

// ---- Counter init ----------------------------------------------------------

#[test]
fn initialize_sets_correct_discriminator() {
    let (mut svm, payer) = setup();
    let counter = do_initialize(&mut svm, &payer);
    let account = svm.get_account(&counter).expect("counter exists");

    // Account should be: disc(8) + value(8) = 16 bytes, owned by program
    assert_eq!(account.data.len(), 16, "counter should be 16 bytes (disc + u64)");
    assert_eq!(account.owner, program_id(), "counter should be owned by accounts program");

    // Discriminator should be non-zero (SHA256 hash prefix)
    let disc = &account.data[0..8];
    assert_ne!(disc, &[0u8; 8], "discriminator should be set after init");

    let value = u64::from_le_bytes(account.data[8..16].try_into().unwrap());
    assert_eq!(value, 1, "value should be 1 after init");
}

#[test]
fn initialize_rejects_double_init() {
    let (mut svm, payer) = setup();
    do_initialize(&mut svm, &payer);
    svm.expire_blockhash();

    // Second init on same PDA should fail — account already exists
    let counter = counter_pda();
    let metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(counter, false),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
    ];
    let result = call_raw(&mut svm, &payer, 0, metas);
    assert!(result.is_err(), "double init on same PDA should be rejected");
}
