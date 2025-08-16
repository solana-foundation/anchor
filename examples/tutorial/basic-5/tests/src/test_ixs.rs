use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signer},
        system_program,
    },
    Client, Cluster, Program,
};
use basic_5::{accounts, instruction, ActionState};
use std::sync::Arc;

fn setup_program() -> (Client<Arc<Keypair>>, Program<Arc<Keypair>>, Keypair) {
    let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
    let payer = Arc::new(read_keypair_file(&anchor_wallet).unwrap());
    let client = Client::new_with_options(
        Cluster::Localnet,
        Arc::clone(&payer),
        CommitmentConfig::confirmed(),
    );
    let program = client.program(basic_5::id()).unwrap();

    (client, program, payer.insecure_clone())
}

fn find_action_state_pda(program_id: &Pubkey, user_pubkey: &Pubkey) -> Pubkey {
    let seed = b"action-state";
    let (pda, _bump) = Pubkey::find_program_address(&[seed, &user_pubkey.to_bytes()], program_id);
    pda
}

#[test]
fn test_robot_actions() {
    let (_client, program, payer) = setup_program();
    let user = payer.pubkey();
    let action_state = find_action_state_pda(&program.id(), &user);

    let _create = program
        .request()
        .accounts(accounts::Create {
            action_state,
            user,
            system_program: system_program::id(),
        })
        .args(instruction::Create {})
        .send()
        .expect("Failed to create instruction");

    let result: ActionState = program
        .account(action_state)
        .expect("Failed to fetch action state account");

    assert_eq!(result.action, 0);

    let _walk = program
        .request()
        .accounts(accounts::Walk { action_state, user })
        .args(instruction::Walk {})
        .send()
        .expect("Failed to create walk instruction");

    let result: ActionState = program
        .account(action_state)
        .expect("Failed to fetch action state account");

    assert_eq!(result.action, 1);

    let _run = program
        .request()
        .accounts(accounts::Run { action_state, user })
        .args(instruction::Run {})
        .send()
        .expect("Failed to create run instruction");

    let result: ActionState = program
        .account(action_state)
        .expect("Failed to fetch action state account");

    assert_eq!(result.action, 2);

    let _jump = program
        .request()
        .accounts(accounts::Jump { action_state, user })
        .args(instruction::Jump {})
        .send()
        .expect("Failed to create jump instruction");

    let result: ActionState = program
        .account(action_state)
        .expect("Failed to fetch action state account");

    assert_eq!(result.action, 3);

    let _reset = program
        .request()
        .accounts(accounts::Reset { action_state, user })
        .args(instruction::Reset {})
        .send()
        .expect("Failed to create reset instruction");

    let result: ActionState = program
        .account(action_state)
        .expect("Failed to fetch action state account");

    assert_eq!(result.action, 0);
}
