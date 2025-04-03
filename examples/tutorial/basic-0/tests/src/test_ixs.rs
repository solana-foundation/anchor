use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::{read_keypair_file, Keypair},
    },
    Client, Cluster, Program,
};
use basic_0::{accounts, instruction};
use std::sync::Arc;

fn setup_program() -> (Client<Arc<Keypair>>, Program<Arc<Keypair>>, Keypair) {
    let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
    let payer = Arc::new(read_keypair_file(&anchor_wallet).unwrap());
    let client = Client::new_with_options(
        Cluster::Localnet,
        Arc::clone(&payer),
        CommitmentConfig::confirmed(),
    );
    let program = client.program(basic_0::id()).unwrap();

    (client, program, payer.insecure_clone())
}

#[test]
fn test_initialize() {
    let (_client, program, _authority) = setup_program();

    let tx = program
        .request()
        .accounts(accounts::Initialize {})
        .args(instruction::Initialize {})
        .send()
        .expect("Failed to send initialize account transaction");

    println!("Initialize transaction signature: {}", tx);
}
