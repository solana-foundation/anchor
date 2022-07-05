#![cfg(feature = "test-bpf")]

use {
    anchor_client::{
        anchor_lang::Discriminator,
        solana_sdk::{
            account::Account,
            commitment_config::CommitmentConfig,
            pubkey::Pubkey,
            signature::{Keypair, Signer},
            transaction::Transaction,
        },
        Client, Cluster,
    },
    solana_program_test::{tokio, ProgramTest},
    std::rc::Rc,
};

#[tokio::test]
async fn update_foo() {
    let authority = Keypair::new();
    let foo_pubkey = Pubkey::new_unique();
    let foo_account = {
        let mut foo_data = Vec::new();
        foo_data.extend_from_slice(&zero_copy::Foo::discriminator());
        foo_data.extend_from_slice(bytemuck::bytes_of(&zero_copy::Foo {
            authority: authority.pubkey(),
            ..zero_copy::Foo::default()
        }));

        Account {
            lamports: 1,
            data: foo_data,
            owner: zero_copy::id(),
            ..Account::default()
        }
    };

    let mut pt = ProgramTest::new("zero_copy", zero_copy::id(), None);
    pt.add_account(foo_pubkey, foo_account);
    pt.set_compute_max_units(4157);
    let (mut banks_client, payer, recent_blockhash) = pt.start().await;

    let client = Client::new_with_options(
        Cluster::Debug,
        Rc::new(Keypair::new()),
        CommitmentConfig::processed(),
    );
    let program = client.program(zero_copy::id());
    let update_ix = program
        .request()
        .accounts(zero_copy::accounts::UpdateFoo {
            foo: foo_pubkey,
            authority: authority.pubkey(),
        })
        .args(zero_copy::instruction::UpdateFoo { data: 1u64 })
        .instructions()
        .unwrap()
        .pop()
        .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[update_ix],
        Some(&payer.pubkey()),
        &[&payer, &authority],
        recent_blockhash,
    );

    banks_client.process_transaction(transaction).await.unwrap();
}
