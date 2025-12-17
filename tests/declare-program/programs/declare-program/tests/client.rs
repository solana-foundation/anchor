use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction as SolanaInstruction;

declare_program!(external);

#[test]
pub fn test_instruction_utils() {
    // Incorrect program
    assert!(external::utils::Instruction::try_from_solana_instruction(
        &SolanaInstruction::new_with_bytes(system_program::ID, &[], vec![]),
    )
    .is_err());
    // Incorrect instruction
    assert!(external::utils::Instruction::try_from_solana_instruction(
        &SolanaInstruction::new_with_bytes(external::ID, &[], vec![]),
    )
    .is_err());
    // Not enough accounts
    assert!(external::utils::Instruction::try_from_solana_instruction(
        &SolanaInstruction::new_with_bytes(
            external::ID,
            external::client::args::Init::DISCRIMINATOR,
            vec![],
        )
    )
    .is_err());
    // Incorrect account(s)
    assert!(external::utils::Instruction::try_from_solana_instruction(
        &SolanaInstruction::new_with_bytes(
            external::ID,
            external::client::args::Init::DISCRIMINATOR,
            vec![
                AccountMeta::default(),
                AccountMeta::default(),
                AccountMeta::default(),
            ],
        ),
    )
    .is_err());

    // Correct (`init`)
    let authority = Pubkey::from_str_const("Authority1111111111111111111111111111111111");
    let my_account = Pubkey::from_str_const("MyAccount1111111111111111111111111111111111");
    match external::utils::Instruction::try_from_solana_instruction(
        &SolanaInstruction::new_with_bytes(
            external::ID,
            external::client::args::Init::DISCRIMINATOR,
            vec![
                AccountMeta::new(authority, true),
                AccountMeta::new(my_account, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
        ),
    ) {
        Ok(external::utils::Instruction::Init { accounts, args: _ }) => {
            assert_eq!(accounts.authority, authority);
            assert_eq!(accounts.my_account, my_account);
            assert_eq!(accounts.system_program, system_program::ID);
        }
        Ok(_) => panic!("Expected Init instruction variant"),
        Err(e) => panic!("Expected Ok result, got error: {:?}", e),
    };
    // Missing arg
    assert!(external::utils::Instruction::try_from_solana_instruction(
        &SolanaInstruction::new_with_bytes(
            external::ID,
            external::client::args::Update::DISCRIMINATOR,
            vec![
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new(my_account, false),
            ],
        ),
    )
    .is_err());
    // Correct (`update`)
    let expected_args = external::client::args::Update { value: 1 };
    match external::utils::Instruction::try_from_solana_instruction(
        &SolanaInstruction::new_with_bytes(
            external::ID,
            &[
                external::client::args::Update::DISCRIMINATOR,
                &ser(&expected_args),
            ]
            .concat(),
            vec![
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new(my_account, false),
            ],
        ),
    ) {
        Ok(external::utils::Instruction::Update { accounts, args }) => {
            assert_eq!(accounts.authority, authority);
            assert_eq!(accounts.my_account, my_account);
            assert_eq!(args.value, expected_args.value);
        }
        Ok(_) => panic!("Expected Update instruction variant"),
        Err(e) => panic!("Expected Ok result, got error: {:?}", e),
    };

    // Correct (`update_composite`)
    let expected_args = external::client::args::UpdateComposite { value: 2 };
    match external::utils::Instruction::try_from_solana_instruction(
        &SolanaInstruction::new_with_bytes(
            external::ID,
            &[
                external::client::args::UpdateComposite::DISCRIMINATOR,
                &ser(&expected_args),
            ]
            .concat(),
            vec![
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new(my_account, false),
            ],
        ),
    ) {
        Ok(external::utils::Instruction::UpdateComposite { accounts, args }) => {
            assert_eq!(accounts.update.authority, authority);
            assert_eq!(accounts.update.my_account, my_account);
            assert_eq!(args.value, expected_args.value);
        }
        Ok(_) => panic!("Expected UpdateComposite instruction variant"),
        Err(e) => panic!("Expected Ok result, got error: {:?}", e),
    };

    // Correct (`update_non_instruction_composite`)
    let expected_args = external::client::args::UpdateNonInstructionComposite { value: 3 };
    match external::utils::Instruction::try_from_solana_instruction(
        &SolanaInstruction::new_with_bytes(
            external::ID,
            &[
                external::client::args::UpdateNonInstructionComposite::DISCRIMINATOR,
                &ser(&expected_args),
            ]
            .concat(),
            vec![
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new(my_account, false),
                AccountMeta::new_readonly(external::ID, false),
            ],
        ),
    ) {
        Ok(external::utils::Instruction::UpdateNonInstructionComposite { accounts, args }) => {
            assert_eq!(accounts.non_instruction_update.authority, authority);
            assert_eq!(accounts.non_instruction_update.my_account, my_account);
            assert_eq!(accounts.non_instruction_update.program, external::ID);
            assert_eq!(args.value, expected_args.value);
        }
        Ok(_) => panic!("Expected UpdateNonInstructionComposite instruction variant"),
        Err(e) => panic!("Expected Ok result, got error: {:?}", e),
    };

    // Correct (`update_non_instruction_composite2`)
    let expected_args = external::client::args::UpdateNonInstructionComposite2 { value: 4 };
    match external::utils::Instruction::try_from_solana_instruction(
        &SolanaInstruction::new_with_bytes(
            external::ID,
            &[
                external::client::args::UpdateNonInstructionComposite2::DISCRIMINATOR,
                &ser(&expected_args),
            ]
            .concat(),
            vec![
                AccountMeta::new_readonly(external::ID, false),
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new(my_account, false),
                AccountMeta::new_readonly(external::ID, false),
            ],
        ),
    ) {
        Ok(external::utils::Instruction::UpdateNonInstructionComposite2 { accounts, args }) => {
            assert_eq!(accounts.non_instruction_update.program, external::ID);
            assert_eq!(
                accounts
                    .non_instruction_update_with_different_ident
                    .authority,
                authority
            );
            assert_eq!(
                accounts
                    .non_instruction_update_with_different_ident
                    .my_account,
                my_account
            );
            assert_eq!(
                accounts.non_instruction_update_with_different_ident.program,
                external::ID
            );
            assert_eq!(args.value, expected_args.value);
        }
        Ok(_) => panic!("Expected UpdateNonInstructionComposite2 instruction variant"),
        Err(e) => panic!("Expected Ok result, got error: {:?}", e),
    };

    fn ser(val: impl AnchorSerialize) -> Vec<u8> {
        let mut w = vec![];
        val.serialize(&mut w).unwrap();
        w
    }
}

#[test]
pub fn test_account_utils() {
    // Empty
    assert!(external::utils::Account::try_from_bytes(&[]).is_err());

    const DISC: &[u8] = external::accounts::MyAccount::DISCRIMINATOR;

    // Correct discriminator but invalid data
    assert!(external::utils::Account::try_from_bytes(DISC).is_err());

    // Correct discriminator and valid data
    match external::utils::Account::try_from_bytes(&[DISC, &[1, 0, 0, 0]].concat()) {
        Ok(external::utils::Account::MyAccount(my_account)) => assert_eq!(my_account.field, 1),
        Err(e) => panic!("Expected Ok result, got error: {:?}", e),
    }
}

#[test]
pub fn test_event_utils() {
    // Empty
    assert!(external::utils::Event::try_from_bytes(&[]).is_err());

    const DISC: &[u8] = external::events::MyEvent::DISCRIMINATOR;

    // Correct discriminator but invalid data
    assert!(external::utils::Event::try_from_bytes(DISC).is_err());

    // Correct discriminator and valid data
    match external::utils::Event::try_from_bytes(&[DISC, &[1, 0, 0, 0]].concat()) {
        Ok(external::utils::Event::MyEvent(my_event)) => assert_eq!(my_event.value, 1),
        Err(e) => panic!("Expected Ok result, got error: {:?}", e),
    }
}

#[test]
#[cfg(not(feature = "idl-build"))]
pub fn test_error_code_utils() {
    assert_eq!(external::errors::ProgramError::MyNormalError as u32, 6000);
    assert_eq!(
        external::errors::ProgramError::MyErrorWithSpecialOffset as u32,
        6500
    );
}
