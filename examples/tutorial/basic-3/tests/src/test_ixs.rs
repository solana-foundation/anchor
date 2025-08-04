use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::{read_keypair_file, Keypair, Signer},
        system_program,
    },
    Client, Cluster, Program,
};
use std::sync::Arc;

fn setup_programs() -> (
    Client<Arc<Keypair>>,
    Program<Arc<Keypair>>,
    Program<Arc<Keypair>>,
    Keypair,
) {
    let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
    let payer = Arc::new(read_keypair_file(&anchor_wallet).unwrap());
    let client = Client::new_with_options(
        Cluster::Localnet,
        Arc::clone(&payer),
        CommitmentConfig::confirmed(),
    );
    let puppet_master_program = client.program(puppet_master::id()).unwrap();
    let puppet_program = client.program(puppet::id()).unwrap();

    (
        client,
        puppet_master_program,
        puppet_program,
        payer.insecure_clone(),
    )
}

#[test]
fn test_perform_cpi() {
    let (_client, puppet_master_program, puppet_program, authority) = setup_programs();

    let new_puppet_account = Keypair::new();

    let _tx = puppet_program
        .request()
        .accounts(puppet::accounts::Initialize {
            puppet: new_puppet_account.pubkey(),
            user: authority.pubkey(),
            system_program: system_program::id(),
        })
        .args(puppet::instruction::Initialize {})
        .signer(&authority)
        .signer(&new_puppet_account)
        .send()
        .expect("Failed to send initialize puppet account transaction");

    let _tx = puppet_master_program
        .request()
        .accounts(puppet_master::accounts::PullStrings {
            puppet: new_puppet_account.pubkey(),
            puppet_program: puppet_program.id(),
        })
        .args(puppet_master::instruction::PullStrings { data: 111 })
        .send()
        .expect("Failed to send pull strings transaction");

    let puppet_account: puppet::Data = puppet_program
        .account(new_puppet_account.pubkey())
        .expect("Failed to fetch puppet account");

    assert_eq!(puppet_account.data, 111);
}
