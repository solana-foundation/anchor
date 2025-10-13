use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::{read_keypair_file, Keypair, Signer},
        system_program,
    },
    Client, Cluster, Program,
};
use basic_2::{accounts, instruction, Counter};
use std::sync::Arc;

fn setup_program() -> (Client<Arc<Keypair>>, Program<Arc<Keypair>>, Keypair) {
    let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
    let payer = Arc::new(read_keypair_file(&anchor_wallet).unwrap());
    let client = Client::new_with_options(
        Cluster::Localnet,
        Arc::clone(&payer),
        CommitmentConfig::confirmed(),
    );
    let program = client.program(basic_2::id()).unwrap();

    (client, program, payer.insecure_clone())
}

#[test]
fn test_create_counter() {
    let (_client, program, authority) = setup_program();

    let counter = Keypair::new();

    let _tx = program
        .request()
        .accounts(accounts::Create {
            counter: counter.pubkey(),
            user: authority.pubkey(),
            system_program: system_program::id(),
        })
        .args(instruction::Create {
            authority: authority.pubkey(),
        })
        .signer(&counter)
        .send()
        .expect("Failed to send create counter transaction");

    let counter_account: Counter = program
        .account(counter.pubkey())
        .expect("Failed to fetch counter");

    assert_eq!(counter_account.authority, authority.pubkey());
    assert_eq!(counter_account.count, 0);
}

#[test]
fn test_update_counter() {
    let (_client, program, authority) = setup_program();

    let counter = Keypair::new();

    let _tx = program
        .request()
        .accounts(accounts::Create {
            counter: counter.pubkey(),
            user: authority.pubkey(),
            system_program: system_program::id(),
        })
        .args(instruction::Create {
            authority: authority.pubkey(),
        })
        .signer(&counter)
        .send()
        .expect("Failed to send create counter transaction");

    let _tx = program
        .request()
        .accounts(accounts::Increment {
            counter: counter.pubkey(),
            authority: authority.pubkey(),
        })
        .args(instruction::Increment {})
        .send()
        .expect("Failed to send increment counter transaction");

    let counter_account: Counter = program
        .account(counter.pubkey())
        .expect("Failed to fetch counter");

    assert_eq!(counter_account.authority, authority.pubkey());
    assert_eq!(counter_account.count, 1);
}
