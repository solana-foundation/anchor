use {
    anchor_lang_v2::{
        accounts::Account, bytemuck, programs::System,
        solana_program::instruction::Instruction, Id, InstructionData, Space, ToAccountMetas,
    },
    anchor_v2_testing::{Keypair, LiteSVM, Message, Signer, VersionedMessage, VersionedTransaction},
    litesvm::types::{FailedTransactionMetadata, TransactionMetadata},
    prop_amm_v2::{instruction, state::Oracle, UPDATE_AUTHORITY},
};

type TxResult = Result<TransactionMetadata, FailedTransactionMetadata>;

/// Deterministic keypair matching the on-chain `UPDATE_AUTHORITY` constant
/// (ed25519 seed `[7u8; 32]` → pubkey `GmaDrppBC7P5ARKV8g3djiwP89vz1jLK23V2GBjuAEGB`).
fn update_authority() -> Keypair {
    Keypair::new_from_array([7u8; 32])
}

fn setup() -> (LiteSVM, Keypair, Keypair) {
    let program_id = prop_amm_v2::id();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../../../target/deploy/prop_amm_v2.so");
    svm.add_program(program_id, bytes).unwrap();

    let payer = Keypair::new();
    let oracle = Keypair::new();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    // Fund the update authority so it can sign its own transactions
    // (each tx needs a fee payer; using the authority as payer keeps the
    // account layout simple: [oracle, authority] with no extra signers).
    svm.airdrop(&update_authority().pubkey(), 1_000_000_000)
        .unwrap();

    (svm, payer, oracle)
}

fn send(svm: &mut LiteSVM, ix: Instruction, signers: &[&Keypair]) -> TxResult {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&signers[0].pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    svm.send_transaction(tx)
}

fn read_oracle(svm: &LiteSVM, oracle_pubkey: &Keypair) -> Oracle {
    let account = svm
        .get_account(&oracle_pubkey.pubkey())
        .expect("oracle account");
    assert_eq!(
        account.data.len(),
        <Account<Oracle> as Space>::INIT_SPACE,
        "oracle data length",
    );
    *bytemuck::from_bytes::<Oracle>(&account.data[8..])
}

fn init_oracle(svm: &mut LiteSVM, payer: &Keypair, oracle: &Keypair) {
    let ix = Instruction::new_with_bytes(
        prop_amm_v2::id(),
        &instruction::Initialize {}.data(),
        prop_amm_v2::accounts::Initialize {
            payer: payer.pubkey(),
            oracle: oracle.pubkey(),
            system_program: System::id(),
        }
        .to_account_metas(None),
    );
    send(svm, ix, &[payer, oracle]).expect("initialize");
}

// -------------------------------------------------------------------
// initialize — anchor dispatch path, discrim = 1
// -------------------------------------------------------------------
#[test]
fn test_initialize() {
    let (mut svm, payer, oracle) = setup();
    init_oracle(&mut svm, &payer, &oracle);

    let state = read_oracle(&svm, &oracle);
    assert_eq!(
        state.authority.to_bytes(),
        payer.pubkey().to_bytes(),
        "initial authority is payer",
    );
    assert_eq!(state.price, 0);
}

// -------------------------------------------------------------------
// rotate_authority — anchor dispatch path, discrim = 2
// -------------------------------------------------------------------
#[test]
fn test_rotate_authority() {
    let (mut svm, payer, oracle) = setup();
    init_oracle(&mut svm, &payer, &oracle);

    let new_auth = Keypair::new();
    let ix = Instruction::new_with_bytes(
        prop_amm_v2::id(),
        &instruction::RotateAuthority {
            new_authority: new_auth.pubkey().to_bytes(),
        }
        .data(),
        prop_amm_v2::accounts::RotateAuthority {
            oracle: oracle.pubkey(),
            authority: payer.pubkey(),
        }
        .to_account_metas(None),
    );
    send(&mut svm, ix, &[&payer]).expect("rotate");

    let state = read_oracle(&svm, &oracle);
    assert_eq!(state.authority.to_bytes(), new_auth.pubkey().to_bytes());
}

// -------------------------------------------------------------------
// update — asm entrypoint, discrim = 0
// -------------------------------------------------------------------
#[test]
fn test_update_authorized() {
    let (mut svm, payer, oracle) = setup();
    init_oracle(&mut svm, &payer, &oracle);

    let auth = update_authority();
    // Sanity-check that the hardcoded const actually matches the keypair.
    assert_eq!(auth.pubkey().to_bytes(), UPDATE_AUTHORITY.to_bytes());

    let ix = Instruction::new_with_bytes(
        prop_amm_v2::id(),
        &instruction::Update { new_price: 1234 }.data(),
        // Account order must match the asm parser: [oracle (mut), authority (signer)].
        prop_amm_v2::accounts::Update {
            oracle: oracle.pubkey(),
            authority: auth.pubkey(),
        }
        .to_account_metas(None),
    );
    let res = send(&mut svm, ix, &[&auth]).expect("update");
    println!("update CUs: {}", res.compute_units_consumed);

    let state = read_oracle(&svm, &oracle);
    assert_eq!(state.price, 1234);
    // Authority field is left untouched — update only writes `price`.
    assert_eq!(state.authority.to_bytes(), payer.pubkey().to_bytes());
}

#[test]
fn test_update_rejects_wrong_signer() {
    let (mut svm, payer, oracle) = setup();
    init_oracle(&mut svm, &payer, &oracle);

    // Payer is oracle.authority but NOT the hardcoded update authority.
    let ix = Instruction::new_with_bytes(
        prop_amm_v2::id(),
        &instruction::Update { new_price: 9999 }.data(),
        prop_amm_v2::accounts::Update {
            oracle: oracle.pubkey(),
            authority: payer.pubkey(),
        }
        .to_account_metas(None),
    );
    let res = send(&mut svm, ix, &[&payer]);
    assert!(res.is_err(), "wrong signer must be rejected");
    assert_eq!(read_oracle(&svm, &oracle).price, 0);
}

#[test]
fn test_update_rejects_when_not_signer() {
    let (mut svm, _payer, oracle) = setup();
    let payer = update_authority();
    init_oracle(&mut svm, &payer, &oracle);

    // Pass the update authority as a NON-signer account. Since the tx is
    // signed by a different keypair, the runtime will mark the authority
    // slot `is_signer = 0`.
    let another_payer = Keypair::new();
    svm.airdrop(&another_payer.pubkey(), 1_000_000_000).unwrap();

    let mut metas = prop_amm_v2::accounts::Update {
        oracle: oracle.pubkey(),
        authority: payer.pubkey(),
    }
    .to_account_metas(None);
    // Force the authority meta to NOT be a signer.
    metas[1].is_signer = false;

    let ix = Instruction::new_with_bytes(
        prop_amm_v2::id(),
        &instruction::Update { new_price: 5555 }.data(),
        metas,
    );
    let res = send(&mut svm, ix, &[&another_payer]);
    assert!(res.is_err(), "non-signer authority must be rejected");
    assert_eq!(read_oracle(&svm, &oracle).price, 0);
}
