extern crate std;
use {
    quasar_svm::{Account, Instruction, Pubkey, QuasarSvm},
    quasar_vault_client::*,
    std::{println, vec},
};

fn setup() -> QuasarSvm {
    let elf = std::fs::read("../../target/deploy/quasar_vault.so").unwrap();
    QuasarSvm::new().with_program(&crate::ID, &elf)
}

fn signer(address: Pubkey) -> Account {
    quasar_svm::token::create_keyed_system_account(&address, 10_000_000_000)
}

fn empty(address: Pubkey) -> Account {
    Account {
        address,
        lamports: 0,
        data: vec![],
        owner: quasar_svm::system_program::ID,
        executable: false,
    }
}

#[test]
fn test_deposit() {
    let mut svm = setup();

    let user = Pubkey::new_unique();
    let system_program = quasar_svm::system_program::ID;
    let (vault, _) = Pubkey::find_program_address(&[b"vault", user.as_ref()], &crate::ID);

    let deposit_amount: u64 = 1_000_000_000;

    let instruction: Instruction = DepositInstruction {
        user,
        vault,
        system_program,
        amount: deposit_amount,
    }
    .into();

    let result = svm.process_instruction(&instruction, &[signer(user), empty(vault)]);

    assert!(result.is_ok(), "deposit failed: {:?}", result.raw_result);

    let user_after = result.account(&user).unwrap().lamports;
    let vault_after = result.account(&vault).unwrap().lamports;

    assert_eq!(
        user_after,
        10_000_000_000 - deposit_amount,
        "user lamports after deposit"
    );
    assert_eq!(vault_after, deposit_amount, "vault lamports after deposit");

    println!("  DEPOSIT CU: {}", result.compute_units_consumed);
}

#[test]
fn test_withdraw() {
    let mut svm = setup();

    let user = Pubkey::new_unique();
    let (vault, _) = Pubkey::find_program_address(&[b"vault", user.as_ref()], &crate::ID);

    // Pre-fund vault as program-owned (withdraw uses direct lamport
    // manipulation which requires program ownership of the vault PDA).
    let vault_lamports: u64 = 1_000_000_000;
    let withdraw_amount: u64 = 500_000_000;

    let withdraw_ix: Instruction = WithdrawInstruction {
        user,
        vault,
        amount: withdraw_amount,
    }
    .into();

    let result = svm.process_instruction(
        &withdraw_ix,
        &[
            signer(user),
            Account {
                address: vault,
                lamports: vault_lamports,
                data: vec![],
                owner: crate::ID,
                executable: false,
            },
        ],
    );
    assert!(result.is_ok(), "withdraw failed: {:?}", result.raw_result);

    let user_final = result.account(&user).unwrap().lamports;
    let vault_final = result.account(&vault).unwrap().lamports;

    assert_eq!(
        user_final,
        10_000_000_000 + withdraw_amount,
        "user lamports after withdraw"
    );
    assert_eq!(
        vault_final,
        vault_lamports - withdraw_amount,
        "vault lamports after withdraw"
    );

    println!("  WITHDRAW CU: {}", result.compute_units_consumed);
}
