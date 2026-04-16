use {
    anchor_lang::solana_program::instruction::AccountMeta,
    anchor_lang_v2::InstructionData,
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

#[test]
fn test_direct_set_data() {
    let (mut svm, payer) = setup();
    let (data_pda, _bump) = Pubkey::find_program_address(&[b"data"], &callee_id());

    // Initialize.
    let init_data = callee::instruction::Initialize {}.data();
    let init_metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(data_pda, false),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
    ];
    send_instruction(&mut svm, callee_id(), init_data, init_metas, &payer, &[])
        .expect("initialize should succeed");

    // Set data directly.
    let value: u64 = 99;
    let set_data = callee::instruction::SetData { value }.data();
    let set_metas = vec![AccountMeta::new(data_pda, false)];
    send_instruction(&mut svm, callee_id(), set_data, set_metas, &payer, &[])
        .expect("set_data should succeed");

    // Verify.
    let account = svm.get_account(&data_pda).expect("data account should exist");
    let stored_value = u64::from_le_bytes(account.data[8..16].try_into().unwrap());
    assert_eq!(stored_value, 99);
}

#[test]
fn test_cpi_set_data() {
    let (mut svm, payer) = setup();
    let (data_pda, _bump) = Pubkey::find_program_address(&[b"data"], &callee_id());

    // Step 1: Initialize the data account via callee.
    let init_data = callee::instruction::Initialize {}.data();
    let init_metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(data_pda, false),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
    ];
    send_instruction(&mut svm, callee_id(), init_data, init_metas, &payer, &[])
        .expect("callee::initialize should succeed");

    // Step 2: Call caller which CPIs into callee.
    let value: u64 = 42;
    let proxy_data = caller::instruction::ProxySetData { value }.data();
    let proxy_metas = vec![
        AccountMeta::new(data_pda, false),
        AccountMeta::new_readonly(callee_id(), false),
    ];
    send_instruction(&mut svm, caller_id(), proxy_data, proxy_metas, &payer, &[])
        .expect("caller::proxy_set_data should succeed");

    // Step 3: Verify the CPI wrote the value.
    let account = svm.get_account(&data_pda).expect("data account should exist");
    let stored_value = u64::from_le_bytes(account.data[8..16].try_into().unwrap());
    assert_eq!(stored_value, 42, "CPI should have set value to 42");
}
