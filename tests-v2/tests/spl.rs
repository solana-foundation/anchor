//! Integration tests for `anchor-spl-v2`'s Mint/TokenAccount surface.
//!
//! Exercises the full spl-v2 API:
//!   - Init paths: `#[account(init, mint::*)]` and `#[account(init, token::*)]`
//!   - CPI helpers: mint_to, transfer, transfer_checked, burn, approve,
//!     revoke, close_account
//!   - Accessor methods on Mint and TokenAccount
//!   - Namespaced constraints: mint::decimals, mint::authority,
//!     token::mint, token::authority
//!   - `get_associated_token_address` derivation

use {
    anchor_lang_v2::solana_program::instruction::AccountMeta,
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    spl_token::{
        solana_program::program_pack::Pack,
        state::{Account as SplTokenAccount, Mint as SplMint},
    },
    tests_v2::{build_program, keypair_for, send_instruction},
};

fn program_id() -> Pubkey {
    "SpL1111111111111111111111111111111111111111".parse().unwrap()
}

fn token_program_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap()
}

fn ata_program_id() -> Pubkey {
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL".parse().unwrap()
}

fn setup() -> (LiteSVM, Keypair) {
    let test_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let deploy_dir = test_dir.join("target/deploy");
    build_program(
        test_dir.join("programs/spl").to_str().unwrap(),
        deploy_dir.to_str().unwrap(),
    );

    let mut svm = LiteSVM::new();
    svm.add_program_from_file(program_id(), deploy_dir.join("spl_test.so"))
        .expect("load spl_test program");

    let payer = keypair_for("spl-payer");
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

/// Build the send_instruction args for `init_mint` (discrim = 0).
fn do_init_mint(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_kp: &Keypair,
    authority: &Pubkey,
) {
    let metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(*authority, false),
        AccountMeta::new(mint_kp.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
    ];
    send_instruction(svm, program_id(), vec![0], metas, payer, &[mint_kp])
        .expect("init_mint should succeed");
}

/// Build and dispatch `init_token_account` (discrim = 1).
fn do_init_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    token_kp: &Keypair,
    authority: &Pubkey,
) {
    let metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*authority, false),
        AccountMeta::new(token_kp.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
    ];
    send_instruction(svm, program_id(), vec![1], metas, payer, &[token_kp])
        .expect("init_token_account should succeed");
}

// ---- Init tests ------------------------------------------------------------

#[test]
fn init_mint_creates_mint_with_expected_state() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("mint-authority");
    let mint = Keypair::new();

    do_init_mint(&mut svm, &payer, &mint, &authority.pubkey());

    let account = svm.get_account(&mint.pubkey()).expect("mint exists");
    assert_eq!(account.owner, token_program_id());
    assert_eq!(account.data.len(), SplMint::LEN);

    let state = SplMint::unpack(&account.data).expect("unpack mint");
    assert_eq!(state.decimals, 6);
    assert_eq!(state.supply, 0);
    assert!(state.is_initialized);
    // spl-token uses solana_program::pubkey::Pubkey; compare by bytes.
    let mint_authority_bytes = match state.mint_authority {
        spl_token::solana_program::program_option::COption::Some(pk) => pk.to_bytes(),
        spl_token::solana_program::program_option::COption::None => [0u8; 32],
    };
    assert_eq!(mint_authority_bytes, authority.pubkey().to_bytes());
}

#[test]
fn init_token_account_creates_account_with_expected_state() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-authority");
    let owner = keypair_for("token-owner");
    let mint = Keypair::new();
    let token = Keypair::new();

    do_init_mint(&mut svm, &payer, &mint, &mint_authority.pubkey());
    do_init_token_account(&mut svm, &payer, &mint.pubkey(), &token, &owner.pubkey());

    let account = svm.get_account(&token.pubkey()).expect("token exists");
    assert_eq!(account.owner, token_program_id());
    assert_eq!(account.data.len(), SplTokenAccount::LEN);

    let state = SplTokenAccount::unpack(&account.data).expect("unpack token");
    assert_eq!(state.mint.to_bytes(), mint.pubkey().to_bytes());
    assert_eq!(state.owner.to_bytes(), owner.pubkey().to_bytes());
    assert_eq!(state.amount, 0);
}

// ---- CPI operations --------------------------------------------------------

/// Shared fixture: mint with authority = `authority` + token account owned
/// by `owner` + 100 tokens minted to it.
fn mint_and_fund(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_authority: &Keypair,
    owner: &Pubkey,
    mint_amount: u64,
) -> (Pubkey, Pubkey) {
    let mint = Keypair::new();
    let token = Keypair::new();
    do_init_mint(svm, payer, &mint, &mint_authority.pubkey());
    do_init_token_account(svm, payer, &mint.pubkey(), &token, owner);

    // do_mint_to (discrim = 2)
    let mut data = vec![2];
    data.extend_from_slice(&mint_amount.to_le_bytes());
    let metas = vec![
        AccountMeta::new(mint.pubkey(), false),
        AccountMeta::new(token.pubkey(), false),
        AccountMeta::new_readonly(mint_authority.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    send_instruction(svm, program_id(), data, metas, payer, &[mint_authority])
        .expect("mint_to should succeed");
    (mint.pubkey(), token.pubkey())
}

#[test]
fn mint_to_increases_supply_and_balance() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let (mint, token) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 500);

    let mint_state =
        SplMint::unpack(&svm.get_account(&mint).unwrap().data).expect("unpack mint");
    assert_eq!(mint_state.supply, 500);

    let token_state =
        SplTokenAccount::unpack(&svm.get_account(&token).unwrap().data).expect("unpack token");
    assert_eq!(token_state.amount, 500);
}

#[test]
fn transfer_moves_tokens_between_accounts() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let recipient = keypair_for("recipient");

    let (mint, from) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 1000);
    let to = Keypair::new();
    do_init_token_account(&mut svm, &payer, &mint, &to, &recipient.pubkey());

    // do_transfer (discrim = 3)
    let mut data = vec![3];
    data.extend_from_slice(&250u64.to_le_bytes());
    let metas = vec![
        AccountMeta::new(from, false),
        AccountMeta::new(to.pubkey(), false),
        AccountMeta::new_readonly(owner.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[&owner])
        .expect("transfer should succeed");

    let from_state = SplTokenAccount::unpack(&svm.get_account(&from).unwrap().data).unwrap();
    let to_state = SplTokenAccount::unpack(&svm.get_account(&to.pubkey()).unwrap().data).unwrap();
    assert_eq!(from_state.amount, 750);
    assert_eq!(to_state.amount, 250);
}

#[test]
fn transfer_checked_validates_decimals() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let recipient = keypair_for("recipient");

    let (mint, from) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 1000);
    let to = Keypair::new();
    do_init_token_account(&mut svm, &payer, &mint, &to, &recipient.pubkey());

    // do_transfer_checked (discrim = 4), decimals = 6 (matches init_mint)
    let mut data = vec![4];
    data.extend_from_slice(&100u64.to_le_bytes());
    data.push(6);
    let metas = vec![
        AccountMeta::new(from, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(to.pubkey(), false),
        AccountMeta::new_readonly(owner.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[&owner])
        .expect("transfer_checked should succeed");

    let to_state = SplTokenAccount::unpack(&svm.get_account(&to.pubkey()).unwrap().data).unwrap();
    assert_eq!(to_state.amount, 100);
}

#[test]
fn burn_reduces_supply_and_balance() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let (mint, token) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 1000);

    // do_burn (discrim = 5)
    let mut data = vec![5];
    data.extend_from_slice(&400u64.to_le_bytes());
    let metas = vec![
        AccountMeta::new(token, false),
        AccountMeta::new(mint, false),
        AccountMeta::new_readonly(owner.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[&owner])
        .expect("burn should succeed");

    let mint_state = SplMint::unpack(&svm.get_account(&mint).unwrap().data).unwrap();
    let token_state = SplTokenAccount::unpack(&svm.get_account(&token).unwrap().data).unwrap();
    assert_eq!(mint_state.supply, 600);
    assert_eq!(token_state.amount, 600);
}

#[test]
fn approve_then_revoke_updates_delegate() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let delegate = keypair_for("delegate");
    let (_mint, token) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 1000);

    // do_approve (discrim = 6)
    let mut data = vec![6];
    data.extend_from_slice(&300u64.to_le_bytes());
    let metas = vec![
        AccountMeta::new(token, false),
        AccountMeta::new_readonly(delegate.pubkey(), false),
        AccountMeta::new_readonly(owner.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[&owner])
        .expect("approve should succeed");

    let state = SplTokenAccount::unpack(&svm.get_account(&token).unwrap().data).unwrap();
    let delegate_bytes = match state.delegate {
        spl_token::solana_program::program_option::COption::Some(pk) => pk.to_bytes(),
        spl_token::solana_program::program_option::COption::None => [0u8; 32],
    };
    assert_eq!(delegate_bytes, delegate.pubkey().to_bytes());
    assert_eq!(state.delegated_amount, 300);

    // do_revoke (discrim = 7)
    let metas = vec![
        AccountMeta::new(token, false),
        AccountMeta::new_readonly(owner.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    send_instruction(&mut svm, program_id(), vec![7], metas, &payer, &[&owner])
        .expect("revoke should succeed");

    let state = SplTokenAccount::unpack(&svm.get_account(&token).unwrap().data).unwrap();
    assert!(matches!(
        state.delegate,
        spl_token::solana_program::program_option::COption::None,
    ));
    assert_eq!(state.delegated_amount, 0);
}

#[test]
fn close_account_returns_lamports_to_destination() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let (_mint, token) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 0);

    let dest = keypair_for("dest");
    let dest_before = svm.get_account(&dest.pubkey()).map(|a| a.lamports).unwrap_or(0);
    let token_lamports = svm.get_account(&token).unwrap().lamports;

    // do_close_account (discrim = 8)
    let metas = vec![
        AccountMeta::new(token, false),
        AccountMeta::new(dest.pubkey(), false),
        AccountMeta::new_readonly(owner.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    send_instruction(&mut svm, program_id(), vec![8], metas, &payer, &[&owner])
        .expect("close_account should succeed");

    assert!(svm.get_account(&token).is_none() || svm.get_account(&token).unwrap().lamports == 0);
    let dest_after = svm.get_account(&dest.pubkey()).unwrap().lamports;
    assert_eq!(dest_after, dest_before + token_lamports);
}

// ---- Accessor methods ------------------------------------------------------

#[test]
fn read_mint_touches_all_accessors() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("mint-authority");
    let mint = Keypair::new();
    do_init_mint(&mut svm, &payer, &mint, &authority.pubkey());

    // read_mint (discrim = 9). Program-side assertion is that the call
    // succeeds — CPU-bound accessors exercised along the way show up in
    // the coverage trace as hits on `Mint::supply`/`::decimals`/etc.
    let metas = vec![AccountMeta::new_readonly(mint.pubkey(), false)];
    send_instruction(&mut svm, program_id(), vec![9], metas, &payer, &[])
        .expect("read_mint should succeed");
}

#[test]
fn read_token_account_touches_all_accessors() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let (_mint, token) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 100);

    // read_token_account (discrim = 10). See read_mint rationale.
    let metas = vec![AccountMeta::new_readonly(token, false)];
    send_instruction(&mut svm, program_id(), vec![10], metas, &payer, &[])
        .expect("read_token_account should succeed");
}

// ---- Namespaced constraints ------------------------------------------------

#[test]
fn mint_decimals_constraint_accepts_matching() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("mint-auth");
    let mint = Keypair::new();
    do_init_mint(&mut svm, &payer, &mint, &authority.pubkey());

    // check_mint_decimals (discrim = 11) — init sets decimals = 6, matches.
    let metas = vec![AccountMeta::new(mint.pubkey(), false)];
    send_instruction(&mut svm, program_id(), vec![11], metas, &payer, &[])
        .expect("decimals match should pass");
}

#[test]
fn mint_authority_constraint_accepts_matching() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("mint-auth");
    let mint = Keypair::new();
    do_init_mint(&mut svm, &payer, &mint, &authority.pubkey());

    // check_mint_authority (discrim = 12): expected = `authority`, mint = mint.
    let metas = vec![
        AccountMeta::new_readonly(authority.pubkey(), false),
        AccountMeta::new(mint.pubkey(), false),
    ];
    send_instruction(&mut svm, program_id(), vec![12], metas, &payer, &[])
        .expect("authority match should pass");
}

#[test]
fn mint_authority_constraint_rejects_mismatch() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("mint-auth");
    let wrong = keypair_for("wrong-authority");
    let mint = Keypair::new();
    do_init_mint(&mut svm, &payer, &mint, &authority.pubkey());

    let metas = vec![
        AccountMeta::new_readonly(wrong.pubkey(), false),
        AccountMeta::new(mint.pubkey(), false),
    ];
    let result = send_instruction(&mut svm, program_id(), vec![12], metas, &payer, &[]);
    assert!(result.is_err(), "mismatched authority should reject");
}

#[test]
fn token_mint_constraint_accepts_matching() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let (mint, token) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 0);

    // check_token_mint (discrim = 13): pass the actual mint.
    let metas = vec![
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(token, false),
    ];
    send_instruction(&mut svm, program_id(), vec![13], metas, &payer, &[])
        .expect("token::mint match should pass");
}

#[test]
fn token_authority_constraint_accepts_matching() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let (_mint, token) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 0);

    // check_token_authority (discrim = 14): expected = owner.
    let metas = vec![
        AccountMeta::new_readonly(owner.pubkey(), false),
        AccountMeta::new(token, false),
    ];
    send_instruction(&mut svm, program_id(), vec![14], metas, &payer, &[])
        .expect("token::authority match should pass");
}

// ---- ATA derivation --------------------------------------------------------

#[test]
fn check_ata_accepts_canonical_address() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let wallet = keypair_for("ata-wallet");

    let mint = Keypair::new();
    do_init_mint(&mut svm, &payer, &mint, &mint_authority.pubkey());

    // Derive the canonical ATA and create it via our program.
    let ata = Pubkey::find_program_address(
        &[
            wallet.pubkey().as_ref(),
            token_program_id().as_ref(),
            mint.pubkey().as_ref(),
        ],
        &ata_program_id(),
    )
    .0;

    // Use the ATA program's Create instruction (idempotent create) so the
    // on-chain account matches the address the program derives.
    let create_ata_data = vec![0u8]; // Create discriminator
    let create_metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(ata, false),
        AccountMeta::new_readonly(wallet.pubkey(), false),
        AccountMeta::new_readonly(mint.pubkey(), false),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    send_instruction(
        &mut svm,
        ata_program_id(),
        create_ata_data,
        create_metas,
        &payer,
        &[],
    )
    .expect("create ATA should succeed");

    // check_ata (discrim = 15) — passes if derivation matches `vault` addr.
    let metas = vec![
        AccountMeta::new_readonly(wallet.pubkey(), false),
        AccountMeta::new_readonly(mint.pubkey(), false),
        AccountMeta::new_readonly(ata, false),
    ];
    send_instruction(&mut svm, program_id(), vec![15], metas, &payer, &[])
        .expect("canonical ATA should pass");
}
