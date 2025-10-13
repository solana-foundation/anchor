use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::{read_keypair_file, Keypair, Signer},
        system_program,
    },
    Client, Cluster, Program,
};
use basic_1::{accounts, instruction, MyAccount};
use std::sync::Arc;

fn setup_program() -> (Client<Arc<Keypair>>, Program<Arc<Keypair>>, Keypair) {
    let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
    let payer = Arc::new(read_keypair_file(&anchor_wallet).unwrap());
    let client = Client::new_with_options(
        Cluster::Localnet,
        Arc::clone(&payer),
        CommitmentConfig::confirmed(),
    );
    let program = client.program(basic_1::id()).unwrap();

    (client, program, payer.insecure_clone())
}

#[test]
fn test_create_and_initialize_account() {
    let (_client, program, authority) = setup_program();

    let my_account = Keypair::new();

    let _tx = program
        .request()
        .accounts(accounts::Initialize {
            my_account: my_account.pubkey(),
            user: authority.pubkey(),
            system_program: system_program::id(),
        })
        .args(instruction::Initialize { data: 1234 })
        .signer(&my_account)
        .send()
        .expect("Failed to send initialize account transaction");

    let account: MyAccount = program
        .account(my_account.pubkey())
        .expect("Failed to fetch account");

    assert_eq!(account.data, 1234);
}

#[test]
fn test_update_account() {
    let (_client, program, authority) = setup_program();

    let my_account = Keypair::new();

    let _tx = program
        .request()
        .accounts(accounts::Initialize {
            my_account: my_account.pubkey(),
            user: authority.pubkey(),
            system_program: system_program::id(),
        })
        .args(instruction::Initialize { data: 1234 })
        .signer(&my_account)
        .send()
        .expect("Failed to send initialize account transaction");

    let _tx = program
        .request()
        .accounts(accounts::Update {
            my_account: my_account.pubkey(),
        })
        .args(instruction::Update { data: 4321 })
        .send()
        .expect("Failed to send update account transaction");

    let account: MyAccount = program
        .account(my_account.pubkey())
        .expect("Failed to fetch account");

    assert_eq!(account.data, 4321);
}
