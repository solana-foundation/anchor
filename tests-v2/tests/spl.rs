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
    solana_account::Account,
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    spl_token::{
        solana_program::{
            program_option::COption, program_pack::Pack, pubkey::Pubkey as SplPubkey,
        },
        state::{Account as SplTokenAccount, AccountState, Mint as SplMint},
    },
    tests_v2::{build_program, keypair_for, send_instruction},
};

fn program_id() -> Pubkey {
    "SpL1111111111111111111111111111111111111111".parse().unwrap()
}

fn token_program_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap()
}

fn token_2022_program_id() -> Pubkey {
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".parse().unwrap()
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

// ---- CPI negative tests ---------------------------------------------------

#[test]
fn transfer_checked_rejects_wrong_decimals() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let recipient = keypair_for("recipient");

    let (mint, from) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 1000);
    let to = Keypair::new();
    do_init_token_account(&mut svm, &payer, &mint, &to, &recipient.pubkey());

    // Pass decimals=9 when mint has decimals=6
    let mut data = vec![4];
    data.extend_from_slice(&100u64.to_le_bytes());
    data.push(9); // wrong
    let metas = vec![
        AccountMeta::new(from, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(to.pubkey(), false),
        AccountMeta::new_readonly(owner.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    let result = send_instruction(&mut svm, program_id(), data, metas, &payer, &[&owner]);
    assert!(result.is_err(), "wrong decimals should be rejected by SPL token program");
}

#[test]
fn transfer_rejects_insufficient_balance() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let recipient = keypair_for("recipient");

    let (mint, from) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 100);
    let to = Keypair::new();
    do_init_token_account(&mut svm, &payer, &mint, &to, &recipient.pubkey());

    // Try to transfer 200 when only 100 available
    let mut data = vec![3];
    data.extend_from_slice(&200u64.to_le_bytes());
    let metas = vec![
        AccountMeta::new(from, false),
        AccountMeta::new(to.pubkey(), false),
        AccountMeta::new_readonly(owner.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    let result = send_instruction(&mut svm, program_id(), data, metas, &payer, &[&owner]);
    assert!(result.is_err(), "overdraft should be rejected");
}

#[test]
fn mint_to_rejects_wrong_authority() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let impostor = keypair_for("impostor");
    svm.airdrop(&impostor.pubkey(), 1_000_000_000).unwrap();
    let owner = keypair_for("owner");

    let (mint, token) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 0);

    let mut data = vec![2];
    data.extend_from_slice(&500u64.to_le_bytes());
    let metas = vec![
        AccountMeta::new(mint, false),
        AccountMeta::new(token, false),
        AccountMeta::new_readonly(impostor.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    let result = send_instruction(&mut svm, program_id(), data, metas, &payer, &[&impostor]);
    assert!(result.is_err(), "wrong mint authority should be rejected");

    // Verify nothing was minted
    let state = SplTokenAccount::unpack(&svm.get_account(&token).unwrap().data).unwrap();
    assert_eq!(state.amount, 0, "balance should be unchanged after failed mint");
}

#[test]
fn burn_rejects_wrong_authority() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let impostor = keypair_for("impostor");
    svm.airdrop(&impostor.pubkey(), 1_000_000_000).unwrap();

    let (mint, token) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 500);

    let mut data = vec![5];
    data.extend_from_slice(&100u64.to_le_bytes());
    let metas = vec![
        AccountMeta::new(token, false),
        AccountMeta::new(mint, false),
        AccountMeta::new_readonly(impostor.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    let result = send_instruction(&mut svm, program_id(), data, metas, &payer, &[&impostor]);
    assert!(result.is_err(), "wrong burn authority should be rejected");

    let state = SplTokenAccount::unpack(&svm.get_account(&token).unwrap().data).unwrap();
    assert_eq!(state.amount, 500, "balance should be unchanged after failed burn");
}

#[test]
fn close_account_rejects_non_zero_balance() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let (_mint, token) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 500);

    let dest = keypair_for("dest");
    let metas = vec![
        AccountMeta::new(token, false),
        AccountMeta::new(dest.pubkey(), false),
        AccountMeta::new_readonly(owner.pubkey(), true),
        AccountMeta::new_readonly(token_program_id(), false),
    ];
    let result = send_instruction(&mut svm, program_id(), vec![8], metas, &payer, &[&owner]);
    assert!(result.is_err(), "closing account with non-zero balance should be rejected");
}

// ---- Constraint negative tests --------------------------------------------

#[test]
fn mint_decimals_constraint_rejects_mismatch() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("mint-auth");

    // Create a normal mint with decimals=6 via our program
    let mint = Keypair::new();
    do_init_mint(&mut svm, &payer, &mint, &authority.pubkey());

    // Mutate the on-chain data to change decimals from 6 to 9.
    // Mint layout: [authority_flag:4][authority:32][supply:8][decimals:1][is_init:1][...]
    // decimals is at byte offset 44.
    let mut account = svm.get_account(&mint.pubkey()).expect("mint exists");
    assert_eq!(account.data[44], 6, "sanity: original decimals should be 6");
    account.data[44] = 9;
    svm.set_account(mint.pubkey(), account).unwrap();

    // check_mint_decimals (discrim=11) expects decimals=6
    let metas = vec![AccountMeta::new(mint.pubkey(), false)];
    let result = send_instruction(&mut svm, program_id(), vec![11], metas, &payer, &[]);
    assert!(result.is_err(), "mint with decimals=9 should fail decimals=6 constraint");
}

#[test]
fn token_mint_constraint_rejects_mismatch() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");

    // Create two different mints
    let (mint_a, _token_a) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 0);
    let (_mint_b, token_b) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 0);

    // check_token_mint (discrim=13) with mint_a but token_b (which belongs to mint_b)
    let metas = vec![
        AccountMeta::new_readonly(mint_a, false),
        AccountMeta::new(token_b, false),
    ];
    let result = send_instruction(&mut svm, program_id(), vec![13], metas, &payer, &[]);
    assert!(result.is_err(), "token::mint mismatch should be rejected");
}

#[test]
fn token_authority_constraint_rejects_mismatch() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mint-auth");
    let owner = keypair_for("owner");
    let wrong = keypair_for("wrong-auth");

    let (_mint, token) = mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 0);

    // check_token_authority (discrim=14) with wrong expected authority
    let metas = vec![
        AccountMeta::new_readonly(wrong.pubkey(), false),
        AccountMeta::new(token, false),
    ];
    let result = send_instruction(&mut svm, program_id(), vec![14], metas, &payer, &[]);
    assert!(result.is_err(), "token::authority mismatch should be rejected");
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

// ---- Token-2022 seeding helpers --------------------------------------------
//
// Build raw account data for Token-2022 extended mints and token accounts and
// drop it directly into the SVM via `set_account`. No CPI to Token-2022 is
// required because `InterfaceAccount` validation is ownership + length only;
// extension parsing is pure byte-level reading.

/// Convert our `solana_pubkey::Pubkey` into the `spl_token`-flavoured one so
/// `SplMint`/`SplTokenAccount` `pack` calls accept it.
fn to_spl(pk: &Pubkey) -> SplPubkey {
    SplPubkey::new_from_array(pk.to_bytes())
}

/// Build an 82-byte legacy `SplMint` state. Suitable for use as the base of
/// either a Token-2022 extended mint or a plain (legacy) mint account.
fn pack_base_mint(authority: &Pubkey, decimals: u8, supply: u64) -> [u8; SplMint::LEN] {
    let mint_state = SplMint {
        mint_authority: COption::Some(to_spl(authority)),
        supply,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    let mut base = [0u8; SplMint::LEN];
    mint_state.pack_into_slice(&mut base);
    base
}

/// Like `pack_base_mint` but with `freeze_authority` set to `Some(freeze)`.
fn pack_base_mint_with_freeze(
    authority: &Pubkey,
    freeze: &Pubkey,
    decimals: u8,
    supply: u64,
) -> [u8; SplMint::LEN] {
    let mint_state = SplMint {
        mint_authority: COption::Some(to_spl(authority)),
        supply,
        decimals,
        is_initialized: true,
        freeze_authority: COption::Some(to_spl(freeze)),
    };
    let mut base = [0u8; SplMint::LEN];
    mint_state.pack_into_slice(&mut base);
    base
}

/// Build a 165-byte legacy `SplTokenAccount` state.
fn pack_base_token_account(mint: &Pubkey, owner: &Pubkey, amount: u64) -> [u8; SplTokenAccount::LEN] {
    let state = SplTokenAccount {
        mint: to_spl(mint),
        owner: to_spl(owner),
        amount,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    let mut base = [0u8; SplTokenAccount::LEN];
    state.pack_into_slice(&mut base);
    base
}

/// Append a single TLV entry to `buf`: `u16_le type | u16_le length | value`.
fn push_tlv(buf: &mut Vec<u8>, ext_type: u16, value: &[u8]) {
    buf.extend_from_slice(&ext_type.to_le_bytes());
    buf.extend_from_slice(&(value.len() as u16).to_le_bytes());
    buf.extend_from_slice(value);
}

/// Build data for a Token-2022 extended mint: 82-byte base + zero pad to 165 +
/// `AccountType::Mint = 1` at byte 165 + caller-provided TLV region.
fn build_mint_data(
    authority: &Pubkey,
    decimals: u8,
    supply: u64,
    tlv: &[u8],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(166 + tlv.len());
    data.extend_from_slice(&pack_base_mint(authority, decimals, supply));
    // pad to 165
    data.resize(165, 0);
    data.push(1); // AccountType::Mint
    data.extend_from_slice(tlv);
    data
}

/// Like `build_mint_data` but with `freeze_authority` set to `Some(freeze)`.
fn build_mint_data_with_freeze(
    authority: &Pubkey,
    freeze: &Pubkey,
    decimals: u8,
    supply: u64,
    tlv: &[u8],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(166 + tlv.len());
    data.extend_from_slice(&pack_base_mint_with_freeze(authority, freeze, decimals, supply));
    data.resize(165, 0);
    data.push(1); // AccountType::Mint
    data.extend_from_slice(tlv);
    data
}

/// Build data for a Token-2022 extended token account: 165-byte base +
/// `AccountType::Account = 2` at byte 165 + caller-provided TLV region.
fn build_token_account_data(
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
    tlv: &[u8],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(166 + tlv.len());
    data.extend_from_slice(&pack_base_token_account(mint, owner, amount));
    data.push(2); // AccountType::Account
    data.extend_from_slice(tlv);
    data
}

/// Seed a Token-2022-owned account at `address` with the given raw bytes.
fn seed_token_2022_account(svm: &mut LiteSVM, address: Pubkey, data: Vec<u8>) {
    svm.set_account(
        address,
        Account {
            lamports: 10_000_000,
            data,
            owner: token_2022_program_id(),
            executable: false,
            rent_epoch: 0,
        },
    )
    .expect("seed token-2022 account");
}

// ---- InterfaceAccount read path --------------------------------------------

#[test]
fn read_interface_mint_accepts_legacy_token_owned() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("interface-mint-auth");
    let mint = Keypair::new();
    do_init_mint(&mut svm, &payer, &mint, &authority.pubkey());

    // read_interface_mint (discrim = 16)
    let metas = vec![AccountMeta::new_readonly(mint.pubkey(), false)];
    send_instruction(&mut svm, program_id(), vec![16], metas, &payer, &[])
        .expect("legacy-owned mint should pass interface load");
}

#[test]
fn read_interface_mint_accepts_token_2022_owned() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("t22-mint-auth");
    let mint = Pubkey::new_unique();

    let data = build_mint_data(&authority.pubkey(), 9, 0, &[]);
    seed_token_2022_account(&mut svm, mint, data);

    let metas = vec![AccountMeta::new_readonly(mint, false)];
    send_instruction(&mut svm, program_id(), vec![16], metas, &payer, &[])
        .expect("token-2022-owned mint should pass interface load");
}

#[test]
fn read_interface_mint_rejects_foreign_owner() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("foreign-mint-auth");
    let mint = Pubkey::new_unique();

    // Same bytes as a Token-2022 mint, but owned by an unrelated program.
    let data = build_mint_data(&authority.pubkey(), 6, 0, &[]);
    let foreign_owner = Pubkey::new_unique();
    svm.set_account(
        mint,
        Account {
            lamports: 10_000_000,
            data,
            owner: foreign_owner,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let metas = vec![AccountMeta::new_readonly(mint, false)];
    let result = send_instruction(&mut svm, program_id(), vec![16], metas, &payer, &[]);
    assert!(
        result.is_err(),
        "foreign-owned account should not load as InterfaceAccount<Mint>",
    );
}

#[test]
fn read_interface_token_account_accepts_legacy_and_token_2022() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mix-mint-auth");
    let owner = keypair_for("mix-owner");

    // Legacy branch: reuse the classic mint_and_fund flow.
    let (_mint, legacy_token) =
        mint_and_fund(&mut svm, &payer, &mint_authority, &owner.pubkey(), 10);
    let metas = vec![AccountMeta::new_readonly(legacy_token, false)];
    send_instruction(&mut svm, program_id(), vec![17], metas, &payer, &[])
        .expect("legacy-owned token account should pass interface load");

    // Token-2022 branch: seed a raw extended account.
    let t22_mint = Pubkey::new_unique();
    let t22_token = Pubkey::new_unique();
    seed_token_2022_account(
        &mut svm,
        t22_mint,
        build_mint_data(&mint_authority.pubkey(), 6, 0, &[]),
    );
    seed_token_2022_account(
        &mut svm,
        t22_token,
        build_token_account_data(&t22_mint, &owner.pubkey(), 42, &[]),
    );
    let metas = vec![AccountMeta::new_readonly(t22_token, false)];
    send_instruction(&mut svm, program_id(), vec![17], metas, &payer, &[])
        .expect("token-2022-owned token account should pass interface load");
}

// ---- InterfaceAccount init path --------------------------------------------
//
// Init is hard-wired to the legacy Token program via `pinocchio_token::
// InitializeMint2` / `InitializeAccount3`, so these tests exercise the
// interface codegen while creating legacy-owned accounts. The Token-2022
// init path is a known limitation of the PR.

#[test]
fn init_interface_mint_creates_legacy_mint() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("iface-init-mint-auth");
    let mint = Keypair::new();

    // init_interface_mint (discrim = 18)
    let metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(authority.pubkey(), false),
        AccountMeta::new_readonly(token_program_id(), false),
        AccountMeta::new(mint.pubkey(), true),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
    ];
    send_instruction(&mut svm, program_id(), vec![18], metas, &payer, &[&mint])
        .expect("init_interface_mint should succeed");

    let account = svm.get_account(&mint.pubkey()).expect("mint exists");
    assert_eq!(account.owner, token_program_id());
    assert_eq!(account.data.len(), SplMint::LEN);
    let state = SplMint::unpack(&account.data).expect("unpack mint");
    assert_eq!(state.decimals, 6);
    assert!(state.is_initialized);
}

#[test]
fn init_interface_token_account_creates_legacy_token_account() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("iface-init-token-mint-auth");
    let owner = keypair_for("iface-init-token-owner");
    let mint = Keypair::new();
    let token = Keypair::new();

    // Seed the mint through the non-interface init path so we don't double-
    // cover codegen; `Account<Mint>` coexists with `InterfaceAccount<Mint>`
    // on-chain — the underlying bytes are identical.
    do_init_mint(&mut svm, &payer, &mint, &mint_authority.pubkey());

    // init_interface_token_account (discrim = 19)
    let metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(mint.pubkey(), false),
        AccountMeta::new_readonly(owner.pubkey(), false),
        AccountMeta::new_readonly(token_program_id(), false),
        AccountMeta::new(token.pubkey(), true),
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
    ];
    send_instruction(&mut svm, program_id(), vec![19], metas, &payer, &[&token])
        .expect("init_interface_token_account should succeed");

    let account = svm.get_account(&token.pubkey()).expect("token exists");
    assert_eq!(account.owner, token_program_id());
    assert_eq!(account.data.len(), SplTokenAccount::LEN);
    let state = SplTokenAccount::unpack(&account.data).expect("unpack token");
    assert_eq!(state.mint.to_bytes(), mint.pubkey().to_bytes());
    assert_eq!(state.owner.to_bytes(), owner.pubkey().to_bytes());
}

// ---- Namespaced constraints on InterfaceAccount ----------------------------

/// Shared fixture: a Token-2022-owned mint + token account pair. No
/// extensions — just the base state with the AccountType byte set.
fn seed_t22_mint_and_token(
    svm: &mut LiteSVM,
    mint_authority: &Pubkey,
    owner: &Pubkey,
) -> (Pubkey, Pubkey) {
    let mint = Pubkey::new_unique();
    let token = Pubkey::new_unique();
    seed_token_2022_account(svm, mint, build_mint_data(mint_authority, 6, 0, &[]));
    seed_token_2022_account(
        svm,
        token,
        build_token_account_data(&mint, owner, 0, &[]),
    );
    (mint, token)
}

#[test]
fn interface_token_mint_constraint_accepts_matching() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("iface-tm-auth");
    let owner = keypair_for("iface-tm-owner");
    let (mint, token) =
        seed_t22_mint_and_token(&mut svm, &mint_authority.pubkey(), &owner.pubkey());

    // check_interface_token_mint (discrim = 20)
    let metas = vec![
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(token, false),
    ];
    send_instruction(&mut svm, program_id(), vec![20], metas, &payer, &[])
        .expect("matching token::mint should pass");
}

#[test]
fn interface_token_mint_constraint_rejects_mismatch() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("iface-tm-rej-auth");
    let owner = keypair_for("iface-tm-rej-owner");
    let (_real_mint, token) =
        seed_t22_mint_and_token(&mut svm, &mint_authority.pubkey(), &owner.pubkey());

    // A different mint (wrong one).
    let other_mint = Pubkey::new_unique();
    seed_token_2022_account(
        &mut svm,
        other_mint,
        build_mint_data(&mint_authority.pubkey(), 6, 0, &[]),
    );

    let metas = vec![
        AccountMeta::new_readonly(other_mint, false),
        AccountMeta::new(token, false),
    ];
    let result = send_instruction(&mut svm, program_id(), vec![20], metas, &payer, &[]);
    assert!(result.is_err(), "mismatched token::mint should reject");
}

#[test]
fn interface_token_authority_constraint_accepts_matching() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("iface-ta-auth");
    let owner = keypair_for("iface-ta-owner");
    let (_mint, token) =
        seed_t22_mint_and_token(&mut svm, &mint_authority.pubkey(), &owner.pubkey());

    // check_interface_token_authority (discrim = 21)
    let metas = vec![
        AccountMeta::new_readonly(owner.pubkey(), false),
        AccountMeta::new(token, false),
    ];
    send_instruction(&mut svm, program_id(), vec![21], metas, &payer, &[])
        .expect("matching token::authority should pass");
}

#[test]
fn interface_token_authority_constraint_rejects_mismatch() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("iface-ta-rej-auth");
    let owner = keypair_for("iface-ta-rej-owner");
    let wrong = keypair_for("iface-ta-rej-wrong");
    let (_mint, token) =
        seed_t22_mint_and_token(&mut svm, &mint_authority.pubkey(), &owner.pubkey());

    let metas = vec![
        AccountMeta::new_readonly(wrong.pubkey(), false),
        AccountMeta::new(token, false),
    ];
    let result = send_instruction(&mut svm, program_id(), vec![21], metas, &payer, &[]);
    assert!(result.is_err(), "mismatched token::authority should reject");
}

#[test]
fn interface_token_program_constraint_accepts_token_2022() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("iface-tp-auth");
    let owner = keypair_for("iface-tp-owner");
    let (_mint, token) =
        seed_t22_mint_and_token(&mut svm, &mint_authority.pubkey(), &owner.pubkey());

    // check_interface_token_program (discrim = 22): expected = Token-2022.
    let metas = vec![
        AccountMeta::new_readonly(token_2022_program_id(), false),
        AccountMeta::new(token, false),
    ];
    send_instruction(&mut svm, program_id(), vec![22], metas, &payer, &[])
        .expect("token-2022-owned account should match Token-2022 program id");
}

#[test]
fn interface_token_program_constraint_rejects_wrong_program() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("iface-tp-rej-auth");
    let owner = keypair_for("iface-tp-rej-owner");
    let (_mint, token) =
        seed_t22_mint_and_token(&mut svm, &mint_authority.pubkey(), &owner.pubkey());

    // expected = legacy Token, actual owner = Token-2022 → reject.
    let metas = vec![
        AccountMeta::new_readonly(token_program_id(), false),
        AccountMeta::new(token, false),
    ];
    let result = send_instruction(&mut svm, program_id(), vec![22], metas, &payer, &[]);
    assert!(
        result.is_err(),
        "legacy Token expected vs Token-2022 owner should reject",
    );
}

#[test]
fn interface_mint_authority_constraint_accepts_matching() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("iface-ma-auth");
    let mint = Pubkey::new_unique();
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&authority.pubkey(), 6, 0, &[]),
    );

    // check_interface_mint_authority (discrim = 23)
    let metas = vec![
        AccountMeta::new_readonly(authority.pubkey(), false),
        AccountMeta::new(mint, false),
    ];
    send_instruction(&mut svm, program_id(), vec![23], metas, &payer, &[])
        .expect("matching mint::authority should pass");
}

#[test]
fn interface_mint_authority_constraint_rejects_mismatch() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("iface-ma-rej-auth");
    let wrong = keypair_for("iface-ma-rej-wrong");
    let mint = Pubkey::new_unique();
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&authority.pubkey(), 6, 0, &[]),
    );

    let metas = vec![
        AccountMeta::new_readonly(wrong.pubkey(), false),
        AccountMeta::new(mint, false),
    ];
    let result = send_instruction(&mut svm, program_id(), vec![23], metas, &payer, &[]);
    assert!(result.is_err(), "mismatched mint::authority should reject");
}

#[test]
fn interface_mint_freeze_authority_constraint_rejects_when_unset() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("iface-mf-auth");
    let expected = keypair_for("iface-mf-expected");
    let mint = Pubkey::new_unique();
    // Base state uses COption::None for freeze_authority.
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&authority.pubkey(), 6, 0, &[]),
    );

    // check_interface_mint_freeze_authority (discrim = 24)
    let metas = vec![
        AccountMeta::new_readonly(expected.pubkey(), false),
        AccountMeta::new(mint, false),
    ];
    let result = send_instruction(&mut svm, program_id(), vec![24], metas, &payer, &[]);
    assert!(
        result.is_err(),
        "freeze_authority unset should fail the constraint",
    );
}

#[test]
fn interface_mint_freeze_authority_constraint_accepts_matching() {
    // Exercises the `Some(addr) if addr == expected` arm of
    // `mint::FreezeAuthorityConstraint::check`.
    let (mut svm, payer) = setup();
    let authority = keypair_for("iface-mf-ok-auth");
    let expected = keypair_for("iface-mf-ok-expected");
    let mint = Pubkey::new_unique();
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data_with_freeze(&authority.pubkey(), &expected.pubkey(), 6, 0, &[]),
    );

    let metas = vec![
        AccountMeta::new_readonly(expected.pubkey(), false),
        AccountMeta::new(mint, false),
    ];
    send_instruction(&mut svm, program_id(), vec![24], metas, &payer, &[])
        .expect("matching mint::freeze_authority should pass");
}

#[test]
fn interface_mint_freeze_authority_constraint_rejects_mismatch() {
    // Exercises the `Some(addr) if addr != expected` (fall-through) arm
    // of `mint::FreezeAuthorityConstraint::check` — distinct from the
    // unset/None case covered above.
    let (mut svm, payer) = setup();
    let authority = keypair_for("iface-mf-rej-auth");
    let expected = keypair_for("iface-mf-rej-expected");
    let other = keypair_for("iface-mf-rej-other");
    let mint = Pubkey::new_unique();
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data_with_freeze(&authority.pubkey(), &other.pubkey(), 6, 0, &[]),
    );

    let metas = vec![
        AccountMeta::new_readonly(expected.pubkey(), false),
        AccountMeta::new(mint, false),
    ];
    let result = send_instruction(&mut svm, program_id(), vec![24], metas, &payer, &[]);
    assert!(
        result.is_err(),
        "mismatched mint::freeze_authority should reject",
    );
}

#[test]
fn interface_mint_decimals_constraint_accepts_matching() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("iface-md-auth");
    let mint = Pubkey::new_unique();
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&authority.pubkey(), 6, 0, &[]),
    );

    // check_interface_mint_decimals (discrim = 25) — expects 6.
    let metas = vec![AccountMeta::new(mint, false)];
    send_instruction(&mut svm, program_id(), vec![25], metas, &payer, &[])
        .expect("matching mint::decimals should pass");
}

#[test]
fn interface_mint_decimals_constraint_rejects_mismatch() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("iface-md-rej-auth");
    let mint = Pubkey::new_unique();
    // Decimals = 9, but constraint expects 6.
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&authority.pubkey(), 9, 0, &[]),
    );

    let metas = vec![AccountMeta::new(mint, false)];
    let result = send_instruction(&mut svm, program_id(), vec![25], metas, &payer, &[]);
    assert!(result.is_err(), "mismatched mint::decimals should reject");
}

#[test]
fn interface_mint_token_program_constraint_accepts_token_2022() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("iface-mtp-auth");
    let mint = Pubkey::new_unique();
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&authority.pubkey(), 6, 0, &[]),
    );

    // check_interface_mint_token_program (discrim = 26): expected = Token-2022.
    let metas = vec![
        AccountMeta::new_readonly(token_2022_program_id(), false),
        AccountMeta::new(mint, false),
    ];
    send_instruction(&mut svm, program_id(), vec![26], metas, &payer, &[])
        .expect("token-2022-owned mint should match Token-2022 program id");
}

#[test]
fn interface_mint_token_program_constraint_rejects_wrong_program() {
    let (mut svm, payer) = setup();
    let authority = keypair_for("iface-mtp-rej-auth");
    let mint = Pubkey::new_unique();
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&authority.pubkey(), 6, 0, &[]),
    );

    // expected = legacy Token, actual owner = Token-2022 → reject.
    let metas = vec![
        AccountMeta::new_readonly(token_program_id(), false),
        AccountMeta::new(mint, false),
    ];
    let result = send_instruction(&mut svm, program_id(), vec![26], metas, &payer, &[]);
    assert!(
        result.is_err(),
        "legacy Token expected vs Token-2022-owned mint should reject",
    );
}

// ---- Token-2022 extension parsing ------------------------------------------

/// Helpers for building TLV values matching the `anchor-spl-v2::extensions`
/// struct layouts exactly. All fields are alignment-1, so raw byte-level
/// construction is safe.

fn tlv_transfer_fee_config(
    authority: &Pubkey,
    withdraw_authority: &Pubkey,
    newer_bps: u16,
    newer_epoch: u64,
    newer_max: u64,
) -> Vec<u8> {
    let mut value = Vec::with_capacity(108);
    value.extend_from_slice(authority.as_ref());
    value.extend_from_slice(withdraw_authority.as_ref());
    value.extend_from_slice(&0u64.to_le_bytes()); // withheld_amount
    // older_transfer_fee: zeroed
    value.extend_from_slice(&[0u8; 8]); // epoch
    value.extend_from_slice(&[0u8; 8]); // max_fee
    value.extend_from_slice(&[0u8; 2]); // basis points
    // newer_transfer_fee
    value.extend_from_slice(&newer_epoch.to_le_bytes());
    value.extend_from_slice(&newer_max.to_le_bytes());
    value.extend_from_slice(&newer_bps.to_le_bytes());
    let mut out = Vec::new();
    push_tlv(&mut out, 1, &value); // TransferFeeConfig
    out
}

fn tlv_metadata_pointer(authority: &Pubkey, metadata: &Pubkey) -> Vec<u8> {
    let mut value = Vec::with_capacity(64);
    value.extend_from_slice(authority.as_ref());
    value.extend_from_slice(metadata.as_ref());
    let mut out = Vec::new();
    push_tlv(&mut out, 18, &value); // MetadataPointer
    out
}

fn tlv_transfer_hook(authority: &Pubkey, program: &Pubkey) -> Vec<u8> {
    let mut value = Vec::with_capacity(64);
    value.extend_from_slice(authority.as_ref());
    value.extend_from_slice(program.as_ref());
    let mut out = Vec::new();
    push_tlv(&mut out, 14, &value); // TransferHook
    out
}

fn tlv_mint_close_authority(authority: &Pubkey) -> Vec<u8> {
    let mut out = Vec::new();
    push_tlv(&mut out, 3, authority.as_ref()); // MintCloseAuthority
    out
}

fn tlv_permanent_delegate(delegate: &Pubkey) -> Vec<u8> {
    let mut out = Vec::new();
    push_tlv(&mut out, 12, delegate.as_ref()); // PermanentDelegate
    out
}

fn tlv_transfer_fee_amount(withheld: u64) -> Vec<u8> {
    let mut out = Vec::new();
    push_tlv(&mut out, 2, &withheld.to_le_bytes()); // TransferFeeAmount
    out
}

fn tlv_transfer_hook_account(transferring: u8) -> Vec<u8> {
    let mut out = Vec::new();
    push_tlv(&mut out, 15, &[transferring]); // TransferHookAccount
    out
}

#[test]
fn transfer_fee_config_extension_round_trips() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("tfc-mint-auth");
    let fee_authority = keypair_for("tfc-fee-auth");
    let withdraw_authority = keypair_for("tfc-withdraw-auth");
    let mint = Pubkey::new_unique();

    let tlv = tlv_transfer_fee_config(
        &fee_authority.pubkey(),
        &withdraw_authority.pubkey(),
        250, // basis points (2.5%)
        4,
        1_000_000,
    );
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&mint_authority.pubkey(), 6, 0, &tlv),
    );

    // read_transfer_fee_config (discrim = 27), expected bps = 250 → pass.
    let mut data = vec![27];
    data.extend_from_slice(&250u16.to_le_bytes());
    let metas = vec![AccountMeta::new_readonly(mint, false)];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("TransferFeeConfig bps should match");

    // Wrong bps → reject.
    let mut data = vec![27];
    data.extend_from_slice(&999u16.to_le_bytes());
    let metas = vec![AccountMeta::new_readonly(mint, false)];
    let result = send_instruction(&mut svm, program_id(), data, metas, &payer, &[]);
    assert!(result.is_err(), "wrong bps should reject");
}

#[test]
fn metadata_pointer_extension_round_trips() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mp-mint-auth");
    let meta_authority = keypair_for("mp-meta-auth");
    let metadata = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let tlv = tlv_metadata_pointer(&meta_authority.pubkey(), &metadata);
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&mint_authority.pubkey(), 6, 0, &tlv),
    );

    // read_metadata_pointer (discrim = 28)
    let mut data = vec![28];
    data.extend_from_slice(&meta_authority.pubkey().to_bytes());
    data.extend_from_slice(&metadata.to_bytes());
    let metas = vec![AccountMeta::new_readonly(mint, false)];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("MetadataPointer should parse and match");
}

#[test]
fn transfer_hook_extension_round_trips() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("th-mint-auth");
    let hook_authority = keypair_for("th-hook-auth");
    let hook_program = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let tlv = tlv_transfer_hook(&hook_authority.pubkey(), &hook_program);
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&mint_authority.pubkey(), 6, 0, &tlv),
    );

    // read_transfer_hook (discrim = 29)
    let mut data = vec![29];
    data.extend_from_slice(&hook_program.to_bytes());
    let metas = vec![AccountMeta::new_readonly(mint, false)];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("TransferHook program id should match");
}

#[test]
fn mint_close_authority_extension_round_trips() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("mca-mint-auth");
    let close_authority = keypair_for("mca-close-auth");
    let mint = Pubkey::new_unique();

    let tlv = tlv_mint_close_authority(&close_authority.pubkey());
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&mint_authority.pubkey(), 6, 0, &tlv),
    );

    // read_mint_close_authority (discrim = 30) — exercises optional_address.
    let mut data = vec![30];
    data.extend_from_slice(&close_authority.pubkey().to_bytes());
    let metas = vec![AccountMeta::new_readonly(mint, false)];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("MintCloseAuthority close authority should match");
}

#[test]
fn permanent_delegate_extension_round_trips() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("pd-mint-auth");
    let delegate = keypair_for("pd-delegate");
    let mint = Pubkey::new_unique();

    let tlv = tlv_permanent_delegate(&delegate.pubkey());
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&mint_authority.pubkey(), 6, 0, &tlv),
    );

    // read_permanent_delegate (discrim = 31)
    let mut data = vec![31];
    data.extend_from_slice(&delegate.pubkey().to_bytes());
    let metas = vec![AccountMeta::new_readonly(mint, false)];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("PermanentDelegate delegate should match");
}

#[test]
fn transfer_fee_amount_extension_round_trips() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("tfa-mint-auth");
    let owner = keypair_for("tfa-owner");
    let mint = Pubkey::new_unique();
    let token = Pubkey::new_unique();

    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&mint_authority.pubkey(), 6, 0, &[]),
    );
    let tlv = tlv_transfer_fee_amount(777);
    seed_token_2022_account(
        &mut svm,
        token,
        build_token_account_data(&mint, &owner.pubkey(), 0, &tlv),
    );

    // read_transfer_fee_amount (discrim = 32)
    let mut data = vec![32];
    data.extend_from_slice(&777u64.to_le_bytes());
    let metas = vec![AccountMeta::new_readonly(token, false)];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("TransferFeeAmount withheld should match");
}

#[test]
fn transfer_hook_account_extension_round_trips() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("tha-mint-auth");
    let owner = keypair_for("tha-owner");
    let mint = Pubkey::new_unique();
    let token = Pubkey::new_unique();

    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&mint_authority.pubkey(), 6, 0, &[]),
    );
    let tlv = tlv_transfer_hook_account(1);
    seed_token_2022_account(
        &mut svm,
        token,
        build_token_account_data(&mint, &owner.pubkey(), 0, &tlv),
    );

    // read_transfer_hook_account (discrim = 33)
    let data = vec![33, 1];
    let metas = vec![AccountMeta::new_readonly(token, false)];
    send_instruction(&mut svm, program_id(), data, metas, &payer, &[])
        .expect("TransferHookAccount transferring should match");
}

#[test]
fn missing_extension_returns_invalid_account_data() {
    let (mut svm, payer) = setup();
    let mint_authority = keypair_for("noext-mint-auth");
    let mint = Pubkey::new_unique();

    // Mint has MintCloseAuthority only — lookup for TransferFeeConfig fails.
    let other = keypair_for("noext-close-auth");
    let tlv = tlv_mint_close_authority(&other.pubkey());
    seed_token_2022_account(
        &mut svm,
        mint,
        build_mint_data(&mint_authority.pubkey(), 6, 0, &tlv),
    );

    let mut data = vec![27]; // read_transfer_fee_config
    data.extend_from_slice(&250u16.to_le_bytes());
    let metas = vec![AccountMeta::new_readonly(mint, false)];
    let result = send_instruction(&mut svm, program_id(), data, metas, &payer, &[]);
    assert!(
        result.is_err(),
        "missing extension should surface InvalidAccountData",
    );
}

// ---- Unit tests for host-side helpers --------------------------------------
//
// `is_some_address` / `optional_address` are `pub fn` on `anchor-spl-v2` and
// can be called directly from the host-side test binary.

#[test]
fn is_some_address_detects_zero_and_nonzero() {
    use anchor_spl_v2::extensions::{is_some_address, optional_address};
    let zero = solana_address::Address::new_from_array([0u8; 32]);
    let mut nonzero_bytes = [0u8; 32];
    nonzero_bytes[31] = 1;
    let nonzero = solana_address::Address::new_from_array(nonzero_bytes);

    assert!(!is_some_address(&zero));
    assert!(is_some_address(&nonzero));
    assert!(optional_address(&zero).is_none());
    assert_eq!(optional_address(&nonzero), Some(&nonzero));
}
