use anchor_lang::prelude::*;

declare_id!("Dec1areProgram11111111111111111111111111111");

declare_program!(external);
use external::program::External;

// Compilation check for legacy IDL (pre Anchor `0.30`)
declare_program!(external_legacy);

// Compilation check for the Raydium AMM v3 program (Anchor v0.29.0)
// https://github.com/raydium-io/raydium-idl/blob/c8507c78618eda1de96ff5e43bd29daefa7e9307/raydium_clmm/amm_v3.json
declare_program!(amm_v3);

#[program]
pub mod declare_program {
    use super::*;

    pub fn cpi(ctx: Context<Cpi>, value: u32) -> Result<()> {
        let cpi_my_account = &mut ctx.accounts.cpi_my_account;
        require_keys_eq!(external::accounts::MyAccount::owner(), external::ID);
        require_eq!(cpi_my_account.field, 0);

        let cpi_ctx = CpiContext::new(
            ctx.accounts.external_program.key(),
            external::cpi::accounts::Update {
                authority: ctx.accounts.authority.to_account_info(),
                my_account: cpi_my_account.to_account_info(),
            },
        );
        external::cpi::update(cpi_ctx, value)?;

        cpi_my_account.reload()?;
        require_eq!(cpi_my_account.field, value);

        Ok(())
    }

    pub fn cpi_composite(ctx: Context<Cpi>, value: u32) -> Result<()> {
        let cpi_my_account = &mut ctx.accounts.cpi_my_account;

        // Composite accounts that's also an instruction
        let cpi_ctx = CpiContext::new(
            ctx.accounts.external_program.key(),
            external::cpi::accounts::UpdateComposite {
                update: external::cpi::accounts::Update {
                    authority: ctx.accounts.authority.to_account_info(),
                    my_account: cpi_my_account.to_account_info(),
                },
            },
        );
        external::cpi::update_composite(cpi_ctx, 42)?;
        cpi_my_account.reload()?;
        require_eq!(cpi_my_account.field, 42);

        // Composite accounts but not an actual instruction
        let cpi_ctx = CpiContext::new(
            ctx.accounts.external_program.key(),
            external::cpi::accounts::UpdateNonInstructionComposite {
                non_instruction_update: external::cpi::accounts::NonInstructionUpdate {
                    authority: ctx.accounts.authority.to_account_info(),
                    my_account: cpi_my_account.to_account_info(),
                    program: ctx.accounts.external_program.to_account_info(),
                },
            },
        );
        external::cpi::update_non_instruction_composite(cpi_ctx, 10)?;
        cpi_my_account.reload()?;
        require_eq!(cpi_my_account.field, 10);

        // Composite accounts but not an actual instruction (intentionally checking multiple times)
        let cpi_ctx = CpiContext::new(
            ctx.accounts.external_program.key(),
            external::cpi::accounts::UpdateNonInstructionComposite2 {
                non_instruction_update: external::cpi::accounts::NonInstructionUpdate2 {
                    program: ctx.accounts.external_program.to_account_info(),
                },
                non_instruction_update_with_different_ident:
                    external::cpi::accounts::NonInstructionUpdate {
                        authority: ctx.accounts.authority.to_account_info(),
                        my_account: cpi_my_account.to_account_info(),
                        program: ctx.accounts.external_program.to_account_info(),
                    },
            },
        );
        external::cpi::update_non_instruction_composite2(cpi_ctx, value)?;
        cpi_my_account.reload()?;
        require_eq!(cpi_my_account.field, value);

        Ok(())
    }

    pub fn account_utils(_ctx: Context<Utils>) -> Result<()> {
        use external::utils::Account;

        // Empty
        if Account::try_from_bytes(&[]).is_ok() {
            return Err(ProgramError::Custom(0).into());
        }

        const DISC: &[u8] = external::accounts::MyAccount::DISCRIMINATOR;

        // Correct discriminator but invalid data
        if Account::try_from_bytes(DISC).is_ok() {
            return Err(ProgramError::Custom(1).into());
        };

        // Correct discriminator and valid data
        match Account::try_from_bytes(&[DISC, &[1, 0, 0, 0]].concat()) {
            Ok(Account::MyAccount(my_account)) => require_eq!(my_account.field, 1),
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }

    pub fn event_utils(_ctx: Context<Utils>) -> Result<()> {
        use external::utils::Event;

        // Empty
        if Event::try_from_bytes(&[]).is_ok() {
            return Err(ProgramError::Custom(0).into());
        }

        const DISC: &[u8] = external::events::MyEvent::DISCRIMINATOR;

        // Correct discriminator but invalid data
        if Event::try_from_bytes(DISC).is_ok() {
            return Err(ProgramError::Custom(1).into());
        };

        // Correct discriminator and valid data
        match Event::try_from_bytes(&[DISC, &[1, 0, 0, 0]].concat()) {
            Ok(Event::MyEvent(my_event)) => require_eq!(my_event.value, 1),
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }

    // TODO: Move utils tests outside of the program
    pub fn instruction_utils(_ctx: Context<Utils>) -> Result<()> {
        use anchor_lang::solana_program::instruction::Instruction as SolanaInstruction;
        use external::utils::Instruction;

        // Incorrect program
        if Instruction::try_from_solana_instruction(&SolanaInstruction::new_with_bytes(
            system_program::ID,
            &[],
            vec![],
        ))
        .is_ok()
        {
            return Err(ProgramError::Custom(0).into());
        };
        // Incorrect instruction
        if Instruction::try_from_solana_instruction(&SolanaInstruction::new_with_bytes(
            external::ID,
            &[],
            vec![],
        ))
        .is_ok()
        {
            return Err(ProgramError::Custom(1).into());
        };
        // Not enough accounts
        if Instruction::try_from_solana_instruction(&SolanaInstruction::new_with_bytes(
            external::ID,
            external::client::args::Init::DISCRIMINATOR,
            vec![],
        ))
        .is_ok()
        {
            return Err(ProgramError::Custom(2).into());
        };
        // Incorrect account(s)
        if Instruction::try_from_solana_instruction(&SolanaInstruction::new_with_bytes(
            external::ID,
            external::client::args::Init::DISCRIMINATOR,
            vec![
                AccountMeta::default(),
                AccountMeta::default(),
                AccountMeta::default(),
            ],
        ))
        .is_ok()
        {
            return Err(ProgramError::Custom(3).into());
        };

        // Correct (`init`)
        let authority = Pubkey::from_str_const("Authority1111111111111111111111111111111111");
        let my_account = Pubkey::from_str_const("MyAccount1111111111111111111111111111111111");
        match Instruction::try_from_solana_instruction(&SolanaInstruction::new_with_bytes(
            external::ID,
            external::client::args::Init::DISCRIMINATOR,
            vec![
                AccountMeta::new(authority, true),
                AccountMeta::new(my_account, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
        )) {
            Ok(Instruction::Init { accounts, .. }) => {
                require_keys_eq!(accounts.authority, authority);
                require_keys_eq!(accounts.my_account, my_account);
                require_keys_eq!(accounts.system_program, system_program::ID);
            }
            Ok(_) => return Err(ProgramError::Custom(4).into()),
            Err(e) => return Err(e.into()),
        };

        // Missing arg
        if Instruction::try_from_solana_instruction(&SolanaInstruction::new_with_bytes(
            external::ID,
            external::client::args::Update::DISCRIMINATOR,
            vec![
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new(my_account, false),
            ],
        ))
        .is_ok()
        {
            return Err(ProgramError::Custom(5).into());
        };

        // Correct (`update`)
        let expected_args = external::client::args::Update { value: 1 };
        match Instruction::try_from_solana_instruction(&SolanaInstruction::new_with_bytes(
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
        )) {
            Ok(Instruction::Update { accounts, args }) => {
                require_keys_eq!(accounts.authority, authority);
                require_keys_eq!(accounts.my_account, my_account);
                require_eq!(args.value, expected_args.value);
            }
            Ok(_) => return Err(ProgramError::Custom(7).into()),
            Err(e) => return Err(e.into()),
        };

        // Correct (`update_composite`)
        let expected_args = external::client::args::UpdateComposite { value: 2 };
        match Instruction::try_from_solana_instruction(&SolanaInstruction::new_with_bytes(
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
        )) {
            Ok(Instruction::UpdateComposite { accounts, args }) => {
                require_keys_eq!(accounts.update.authority, authority);
                require_keys_eq!(accounts.update.my_account, my_account);
                require_eq!(args.value, expected_args.value);
            }
            Ok(_) => return Err(ProgramError::Custom(8).into()),
            Err(e) => return Err(e.into()),
        };

        // Correct (`update_non_instruction_composite`)
        let expected_args = external::client::args::UpdateNonInstructionComposite { value: 3 };
        match Instruction::try_from_solana_instruction(&SolanaInstruction::new_with_bytes(
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
        )) {
            Ok(Instruction::UpdateNonInstructionComposite { accounts, args }) => {
                require_keys_eq!(accounts.non_instruction_update.authority, authority);
                require_keys_eq!(accounts.non_instruction_update.my_account, my_account);
                require_keys_eq!(accounts.non_instruction_update.program, external::ID);
                require_eq!(args.value, expected_args.value);
            }
            Ok(_) => return Err(ProgramError::Custom(9).into()),
            Err(e) => return Err(e.into()),
        };

        // Correct (`update_non_instruction_composite2`)
        let expected_args = external::client::args::UpdateNonInstructionComposite2 { value: 4 };
        match Instruction::try_from_solana_instruction(&SolanaInstruction::new_with_bytes(
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
        )) {
            Ok(Instruction::UpdateNonInstructionComposite2 { accounts, args }) => {
                require_keys_eq!(accounts.non_instruction_update.program, external::ID);
                require_keys_eq!(
                    accounts
                        .non_instruction_update_with_different_ident
                        .authority,
                    authority
                );
                require_keys_eq!(
                    accounts
                        .non_instruction_update_with_different_ident
                        .my_account,
                    my_account
                );
                require_keys_eq!(
                    accounts.non_instruction_update_with_different_ident.program,
                    external::ID
                );
                require_eq!(args.value, expected_args.value);
            }
            Ok(_) => return Err(ProgramError::Custom(10).into()),
            Err(e) => return Err(e.into()),
        };

        fn ser(val: impl AnchorSerialize) -> Vec<u8> {
            let mut w = vec![];
            val.serialize(&mut w).unwrap();
            w
        }

        Ok(())
    }

    pub fn error_code_utils(_ctx: Context<Utils>) -> Result<()> {
        use external::errors::ProgramError;

        require_eq!(ProgramError::MyNormalError as u32, 6000);
        require_eq!(ProgramError::MyErrorWithSpecialOffset as u32, 6500);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Cpi<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub cpi_my_account: Account<'info, external::accounts::MyAccount>,
    pub external_program: Program<'info, External>,
}

#[derive(Accounts)]
pub struct Utils<'info> {
    pub authority: Signer<'info>,
}
