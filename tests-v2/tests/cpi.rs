use {
    anchor_lang_v2::{solana_program::instruction::AccountMeta, InstructionData},
    litesvm::LiteSVM,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    tests_v2::{build_program, keypair_for, send_instruction},
};

fn callee_id() -> Pubkey {
    "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi"
        .parse()
        .unwrap()
}

fn caller_id() -> Pubkey {
    "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR"
        .parse()
        .unwrap()
}

fn setup() -> (LiteSVM, solana_keypair::Keypair) {
    let test_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let deploy_dir = test_dir.join("target/deploy");
    let deploy_str = deploy_dir.to_str().unwrap();

    build_program(
        test_dir.join("programs/callee").to_str().unwrap(),
        deploy_str,
    );
    build_program(
        test_dir.join("programs/caller").to_str().unwrap(),
        deploy_str,
    );

    let mut svm = LiteSVM::new();
    svm.add_program_from_file(callee_id(), &deploy_dir.join("callee.so"))
        .expect("failed to load callee program");
    svm.add_program_from_file(caller_id(), &deploy_dir.join("caller.so"))
        .expect("failed to load caller program");

    let payer = keypair_for("cpi-test-payer");
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    (svm, payer)
}

/// Helper: initialize the callee's data account PDA.
fn init_data_account(
    svm: &mut LiteSVM,
    payer: &solana_keypair::Keypair,
    authority: &solana_keypair::Keypair,
) -> Pubkey {
    let (data_pda, _) = Pubkey::find_program_address(&[b"data"], &callee_id());

    let init_data = callee::instruction::Initialize {}.data();
    let init_metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(data_pda, false),
        AccountMeta::new_readonly(authority.pubkey(), true),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
    ];
    send_instruction(svm, callee_id(), init_data, init_metas, payer, &[authority])
        .expect("callee::initialize should succeed");

    data_pda
}

#[test]
fn test_direct_set_data() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("authority");
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    let data_pda = init_data_account(&mut svm, &payer, &authority);

    // Set data directly via callee.
    let value: u64 = 99;
    let set_data = callee::instruction::SetData { value }.data();
    let set_metas = vec![
        AccountMeta::new(data_pda, false),
        AccountMeta::new_readonly(authority.pubkey(), true),
    ];
    send_instruction(
        &mut svm,
        callee_id(),
        set_data,
        set_metas,
        &payer,
        &[&authority],
    )
    .expect("set_data should succeed");

    // Verify.
    let account = svm
        .get_account(&data_pda)
        .expect("data account should exist");
    let stored_value = u64::from_le_bytes(account.data[8..16].try_into().unwrap());
    assert_eq!(stored_value, 99);
}

#[test]
fn test_cpi_set_data() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("authority");
    svm.airdrop(&authority.pubkey(), 1_000_000_000).unwrap();
    let data_pda = init_data_account(&mut svm, &payer, &authority);

    // Call caller::proxy_set_data which CPIs into callee::set_data.
    // The caller passes both a mutable handle (data) and a read-only
    // handle (authority) through the CpiContext.
    let value: u64 = 42;
    let proxy_data = caller::instruction::ProxySetData { value }.data();
    let proxy_metas = vec![
        AccountMeta::new(data_pda, false),
        AccountMeta::new_readonly(authority.pubkey(), true),
        AccountMeta::new_readonly(callee_id(), false),
    ];
    send_instruction(
        &mut svm,
        caller_id(),
        proxy_data,
        proxy_metas,
        &payer,
        &[&authority],
    )
    .expect("caller::proxy_set_data should succeed");

    // Verify the CPI wrote the value.
    let account = svm
        .get_account(&data_pda)
        .expect("data account should exist");
    let stored_value = u64::from_le_bytes(account.data[8..16].try_into().unwrap());
    assert_eq!(stored_value, 42, "CPI should have set value to 42");
}
