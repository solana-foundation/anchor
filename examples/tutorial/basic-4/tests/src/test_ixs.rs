use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signer},
        system_program,
    },
    Client, Cluster, Program,
};
use basic_4::{accounts, instruction, Counter};
use std::sync::Arc;

fn setup_program() -> (Client<Arc<Keypair>>, Program<Arc<Keypair>>, Keypair) {
    let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
    let payer = Arc::new(read_keypair_file(&anchor_wallet).unwrap());
    let client = Client::new_with_options(
        Cluster::Localnet,
        Arc::clone(&payer),
        CommitmentConfig::confirmed(),
    );
    let program = client.program(basic_4::id()).unwrap();

    (client, program, payer.insecure_clone())
}

fn find_counter_pda(program_id: &Pubkey) -> Pubkey {
    let counter_seed = b"counter";
    let (pda, _bump) = Pubkey::find_program_address(&[counter_seed], program_id);
    pda
}

#[test]
fn test_initialize() {
    let (_client, program, authority) = setup_program();

    let counter_pda = find_counter_pda(&program.id());

    let _tx = program
        .request()
        .accounts(accounts::Initialize {
            counter: counter_pda,
            authority: authority.pubkey(),
            system_program: system_program::id(),
        })
        .args(instruction::Initialize {})
        .send()
        .expect("Failed to send initialize transaction");

    let counter_account: Counter = program
        .account(counter_pda)
        .expect("Failed to fetch counter account");

    assert_eq!(counter_account.count, 0);
}

#[test]
fn test_increment() {
    let (_client, program, authority) = setup_program();

    let counter_pda = find_counter_pda(&program.id());

    let _tx = program
        .request()
        .accounts(accounts::Initialize {
            counter: counter_pda,
            authority: authority.pubkey(),
            system_program: system_program::id(),
        })
        .args(instruction::Initialize {})
        .send()
        .expect("Failed to send initialize transaction");

    let _tx = program
        .request()
        .accounts(accounts::Increment {
            counter: counter_pda,
            authority: authority.pubkey(),
        })
        .args(instruction::Increment {})
        .send()
        .expect("Failed to send increment transaction");

    let counter_account: Counter = program
        .account(counter_pda)
        .expect("Failed to fetch counter account");

    assert_eq!(counter_account.count, 1);
}
