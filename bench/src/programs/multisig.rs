pub mod anchor_v1 {
    use {
        crate::bench::{keypair_for_account, BenchContext, BenchInstruction},
        anchor_lang::{
            prelude::*,
            solana_program::{instruction::AccountMeta, system_program},
            InstructionData, ToAccountMetas,
        },
        anyhow::Result,
        solana_keypair::Keypair,
        solana_signer::Signer,
    };
    
    pub fn build_create_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-create-creator");
        let signer_one = keypair_for_account("multisig-create-signer-one");
        let signer_two = keypair_for_account("multisig-create-signer-two");
        let (config, _) = multisig_config_address(&creator.pubkey());
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
    
        let mut metas = ::multisig_v1::accounts::Create {
            creator: creator.pubkey(),
            config,
            rent: rent::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None);
        metas.push(AccountMeta::new_readonly(signer_one.pubkey(), true));
        metas.push(AccountMeta::new_readonly(signer_two.pubkey(), true));
    
        Ok(BenchInstruction::new(
            ::multisig_v1::instruction::Create { threshold: 2 }.data(),
            metas,
        )
        .with_signers(vec![creator, signer_one, signer_two]))
    }
    
    pub fn build_deposit_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-deposit-creator");
        let signer_one = keypair_for_account("multisig-deposit-signer-one");
        let signer_two = keypair_for_account("multisig-deposit-signer-two");
        let (config, _) = multisig_config_address(&creator.pubkey());
        let (vault, _) = multisig_vault_address(&config);
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        setup_multisig(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        Ok(BenchInstruction::new(
            ::multisig_v1::instruction::Deposit { amount: 1_000_000 }.data(),
            ::multisig_v1::accounts::Deposit {
                depositor: creator.pubkey(),
                config,
                vault,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        )
        .with_signer(creator))
    }
    
    pub fn build_set_label_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-set-label-creator");
        let signer_one = keypair_for_account("multisig-set-label-signer-one");
        let signer_two = keypair_for_account("multisig-set-label-signer-two");
        let (config, _) = multisig_config_address(&creator.pubkey());
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        setup_multisig(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        Ok(BenchInstruction::new(
            ::multisig_v1::instruction::SetLabel {
                label: "bench-multisig".to_owned(),
            }
            .data(),
            ::multisig_v1::accounts::SetLabel {
                creator: creator.pubkey(),
                config,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        )
        .with_signer(creator))
    }
    
    pub fn build_execute_transfer_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-execute-transfer-creator");
        let signer_one = keypair_for_account("multisig-execute-transfer-signer-one");
        let signer_two = keypair_for_account("multisig-execute-transfer-signer-two");
        let (config, _) = multisig_config_address(&creator.pubkey());
        let (vault, _) = multisig_vault_address(&config);
        let recipient = keypair_for_account("multisig-execute-transfer-recipient");
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        ctx.airdrop(&recipient.pubkey(), 1)?;
        setup_multisig(ctx, &creator, &[&signer_one, &signer_two], 2)?;
        ctx.execute_with_signers(
            ::multisig_v1::instruction::Deposit { amount: 1_000_000 }.data(),
            ::multisig_v1::accounts::Deposit {
                depositor: creator.pubkey(),
                config,
                vault,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            &[&creator],
        )?;
    
        let mut metas = ::multisig_v1::accounts::ExecuteTransfer {
            config,
            creator: creator.pubkey(),
            vault,
            recipient: recipient.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None);
        metas.push(AccountMeta::new_readonly(signer_one.pubkey(), true));
        metas.push(AccountMeta::new_readonly(signer_two.pubkey(), true));
    
        Ok(BenchInstruction::new(
            ::multisig_v1::instruction::ExecuteTransfer { amount: 500_000 }.data(),
            metas,
        )
        .with_signers(vec![signer_one, signer_two]))
    }
    
    fn setup_multisig(
        ctx: &mut BenchContext,
        creator: &Keypair,
        signers: &[&Keypair],
        threshold: u8,
    ) -> Result<()> {
        let (config, _) = multisig_config_address(&creator.pubkey());
        let mut metas = ::multisig_v1::accounts::Create {
            creator: creator.pubkey(),
            config,
            rent: rent::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None);
    
        for signer in signers {
            metas.push(AccountMeta::new_readonly(signer.pubkey(), true));
        }
    
        let extra_signers = std::iter::once(creator as &dyn solana_signer::Signer)
            .chain(
                signers
                    .iter()
                    .copied()
                    .map(|signer| signer as &dyn solana_signer::Signer),
            )
            .collect::<Vec<_>>();
    
        ctx.execute_with_signers(
            ::multisig_v1::instruction::Create { threshold }.data(),
            metas,
            &extra_signers,
        )?;
    
        Ok(())
    }
    
    fn multisig_config_address(creator: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"multisig", creator.as_ref()], &::multisig_v1::id())
    }
    
    fn multisig_vault_address(config: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault", config.as_ref()], &::multisig_v1::id())
    }
}

pub mod anchor_v2 {
    use {
        crate::bench::{keypair_for_account, BenchContext, BenchInstruction},
        anchor_lang::solana_program::instruction::AccountMeta,
        anyhow::Result,
        anchor_lang_v2::{Address, InstructionData, ToAccountMetas},
        solana_keypair::Keypair,
        solana_pubkey::Pubkey,
        solana_signer::Signer,
    };
    
    // Convert an anchor-lang-v2 `Address` to a solana-pubkey `Pubkey`.
    fn addr_to_pubkey(addr: &Address) -> Pubkey {
        Pubkey::new_from_array(addr.to_bytes())
    }
    
    // Convert a `Pubkey` to an anchor-lang-v2 `Address`.
    fn pubkey_to_addr(pubkey: &Pubkey) -> Address {
        Address::new_from_array(pubkey.to_bytes())
    }
    
    // Convert v2 AccountMetas to solana AccountMetas.
    fn to_solana_metas(v2_metas: &[anchor_lang_v2::AccountMeta]) -> Vec<AccountMeta> {
        v2_metas
            .iter()
            .map(|m| {
                let pubkey = addr_to_pubkey(&m.address);
                if m.is_writable {
                    AccountMeta::new(pubkey, m.is_signer)
                } else {
                    AccountMeta::new_readonly(pubkey, m.is_signer)
                }
            })
            .collect()
    }
    
    fn multisig_v2_program_id() -> Pubkey {
        multisig_v2::id().into()
    }
    
    fn multisig_v2_config_address(creator: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"multisig", creator.as_ref()],
            &multisig_v2_program_id(),
        )
    }
    
    fn multisig_v2_vault_address(config: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"vault", config.as_ref()],
            &multisig_v2_program_id(),
        )
    }
    
    pub fn build_create_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-create-creator");
        let signer_one = keypair_for_account("multisig-create-signer-one");
        let signer_two = keypair_for_account("multisig-create-signer-two");
        let (config, _) = multisig_v2_config_address(&creator.pubkey());
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
    
        let mut metas = to_solana_metas(
            &multisig_v2::accounts::Create {
                creator: pubkey_to_addr(&creator.pubkey()),
                config: pubkey_to_addr(&config),
                system_program: pubkey_to_addr(&solana_pubkey::Pubkey::new_from_array([0; 32])),
            }
            .to_account_metas(None),
        );
        // Fix system_program address
        metas.last_mut().unwrap().pubkey = anchor_lang::solana_program::system_program::ID;
        // Add remaining accounts (signers)
        metas.push(AccountMeta::new_readonly(signer_one.pubkey(), true));
        metas.push(AccountMeta::new_readonly(signer_two.pubkey(), true));
    
        Ok(BenchInstruction::new(
            multisig_v2::instruction::Create { threshold: 2 }.data(),
            metas,
        )
        .with_signers(vec![creator, signer_one, signer_two]))
    }
    
    pub fn build_deposit_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-deposit-creator");
        let signer_one = keypair_for_account("multisig-deposit-signer-one");
        let signer_two = keypair_for_account("multisig-deposit-signer-two");
        let (config, _) = multisig_v2_config_address(&creator.pubkey());
        let (vault, _) = multisig_v2_vault_address(&config);
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        setup_multisig_v2(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        let mut metas = to_solana_metas(
            &multisig_v2::accounts::Deposit {
                depositor: pubkey_to_addr(&creator.pubkey()),
                config: pubkey_to_addr(&config),
                vault: pubkey_to_addr(&vault),
                system_program: pubkey_to_addr(&solana_pubkey::Pubkey::new_from_array([0; 32])),
            }
            .to_account_metas(None),
        );
        metas.last_mut().unwrap().pubkey = anchor_lang::solana_program::system_program::ID;
    
        Ok(BenchInstruction::new(
            multisig_v2::instruction::Deposit { amount: 1_000_000 }.data(),
            metas,
        )
        .with_signer(creator))
    }
    
    pub fn build_set_label_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-set-label-creator");
        let signer_one = keypair_for_account("multisig-set-label-signer-one");
        let signer_two = keypair_for_account("multisig-set-label-signer-two");
        let (config, _) = multisig_v2_config_address(&creator.pubkey());
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        setup_multisig_v2(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        let mut metas = to_solana_metas(
            &multisig_v2::accounts::SetLabel {
                creator: pubkey_to_addr(&creator.pubkey()),
                config: pubkey_to_addr(&config),
                system_program: pubkey_to_addr(&solana_pubkey::Pubkey::new_from_array([0; 32])),
            }
            .to_account_metas(None),
        );
        metas.last_mut().unwrap().pubkey = anchor_lang::solana_program::system_program::ID;
    
        let label_bytes = b"bench-multisig";
        let mut label = [0u8; 32];
        label[..label_bytes.len()].copy_from_slice(label_bytes);
        Ok(BenchInstruction::new(
            multisig_v2::instruction::SetLabel {
                label_len: label_bytes.len() as u8,
                label,
            }
            .data(),
            metas,
        )
        .with_signer(creator))
    }
    
    pub fn build_execute_transfer_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-execute-transfer-creator");
        let signer_one = keypair_for_account("multisig-execute-transfer-signer-one");
        let signer_two = keypair_for_account("multisig-execute-transfer-signer-two");
        let (config, _) = multisig_v2_config_address(&creator.pubkey());
        let (vault, _) = multisig_v2_vault_address(&config);
        let recipient = keypair_for_account("multisig-execute-transfer-recipient");
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        ctx.airdrop(&recipient.pubkey(), 1)?;
        setup_multisig_v2(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        // Deposit first
        {
            let mut dep_metas = to_solana_metas(
                &multisig_v2::accounts::Deposit {
                    depositor: pubkey_to_addr(&creator.pubkey()),
                    config: pubkey_to_addr(&config),
                    vault: pubkey_to_addr(&vault),
                    system_program: pubkey_to_addr(&solana_pubkey::Pubkey::new_from_array([0; 32])),
                }
                .to_account_metas(None),
            );
            dep_metas.last_mut().unwrap().pubkey = anchor_lang::solana_program::system_program::ID;
            ctx.execute_with_signers(
                multisig_v2::instruction::Deposit { amount: 1_000_000 }.data(),
                dep_metas,
                &[&creator],
            )?;
        }
    
        let mut metas = to_solana_metas(
            &multisig_v2::accounts::ExecuteTransfer {
                config: pubkey_to_addr(&config),
                creator: pubkey_to_addr(&creator.pubkey()),
                vault: pubkey_to_addr(&vault),
                recipient: pubkey_to_addr(&recipient.pubkey()),
                system_program: pubkey_to_addr(&solana_pubkey::Pubkey::new_from_array([0; 32])),
            }
            .to_account_metas(None),
        );
        metas.last_mut().unwrap().pubkey = anchor_lang::solana_program::system_program::ID;
        metas.push(AccountMeta::new_readonly(signer_one.pubkey(), true));
        metas.push(AccountMeta::new_readonly(signer_two.pubkey(), true));
    
        Ok(BenchInstruction::new(
            multisig_v2::instruction::ExecuteTransfer { amount: 500_000 }.data(),
            metas,
        )
        .with_signers(vec![signer_one, signer_two]))
    }
    
    fn setup_multisig_v2(
        ctx: &mut BenchContext,
        creator: &Keypair,
        signers: &[&Keypair],
        threshold: u8,
    ) -> Result<()> {
        let (config, _) = multisig_v2_config_address(&creator.pubkey());
        let mut metas = to_solana_metas(
            &multisig_v2::accounts::Create {
                creator: pubkey_to_addr(&creator.pubkey()),
                config: pubkey_to_addr(&config),
                system_program: pubkey_to_addr(&solana_pubkey::Pubkey::new_from_array([0; 32])),
            }
            .to_account_metas(None),
        );
        metas.last_mut().unwrap().pubkey = anchor_lang::solana_program::system_program::ID;
    
        for signer in signers {
            metas.push(AccountMeta::new_readonly(signer.pubkey(), true));
        }
    
        let extra_signers = std::iter::once(creator as &dyn solana_signer::Signer)
            .chain(
                signers
                    .iter()
                    .copied()
                    .map(|signer| signer as &dyn solana_signer::Signer),
            )
            .collect::<Vec<_>>();
    
        ctx.execute_with_signers(
            multisig_v2::instruction::Create { threshold }.data(),
            metas,
            &extra_signers,
        )?;
    
        Ok(())
    }
}

pub mod pinocchio {
    // Bench runner for the raw-pinocchio multisig program.
    //
    // Like quasar, pinocchio uses a 1-byte instruction discriminator + raw LE
    // args (no borsh/anchor client types), so we construct `AccountMeta`s and
    // ix data bytes manually.
    
    use {
        crate::bench::{keypair_for_account, BenchContext, BenchInstruction},
        anchor_lang::solana_program::instruction::AccountMeta,
        anyhow::Result,
        solana_keypair::Keypair,
        solana_pubkey::Pubkey,
        solana_signer::Signer,
    };
    
    // Shared multisig program id, same as the other four variants.
    fn program_id() -> Pubkey {
        "44444444444444444444444444444444444444444444"
            .parse()
            .unwrap()
    }
    
    fn config_address(creator: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"multisig", creator.as_ref()], &program_id())
    }
    
    fn vault_address(config: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault", config.as_ref()], &program_id())
    }
    
    // Pinocchio uses a 1-byte opcode discriminator + raw LE args.
    fn make_ix_data(disc: u8, args: &[u8]) -> Vec<u8> {
        let mut data = Vec::with_capacity(1 + args.len());
        data.push(disc);
        data.extend_from_slice(args);
        data
    }
    
    pub fn build_create_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-create-creator");
        let signer_one = keypair_for_account("multisig-create-signer-one");
        let signer_two = keypair_for_account("multisig-create-signer-two");
        let (config, _) = config_address(&creator.pubkey());
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
    
        // Pinocchio create accounts: creator, config, system_program + remaining signers.
        let metas = vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
            AccountMeta::new_readonly(signer_one.pubkey(), true),
            AccountMeta::new_readonly(signer_two.pubkey(), true),
        ];
    
        // disc=0, args: threshold(u8)
        Ok(BenchInstruction::new(make_ix_data(0, &[2u8]), metas)
            .with_signers(vec![creator, signer_one, signer_two]))
    }
    
    pub fn build_deposit_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-deposit-creator");
        let signer_one = keypair_for_account("multisig-deposit-signer-one");
        let signer_two = keypair_for_account("multisig-deposit-signer-two");
        let (config, _) = config_address(&creator.pubkey());
        let (vault, _) = vault_address(&config);
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        setup_multisig(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        let metas = vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
        ];
    
        // disc=1, args: amount(u64 LE)
        Ok(
            BenchInstruction::new(make_ix_data(1, &1_000_000u64.to_le_bytes()), metas)
                .with_signer(creator),
        )
    }
    
    pub fn build_set_label_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-set-label-creator");
        let signer_one = keypair_for_account("multisig-set-label-signer-one");
        let signer_two = keypair_for_account("multisig-set-label-signer-two");
        let (config, _) = config_address(&creator.pubkey());
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        setup_multisig(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        let metas = vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
        ];
    
        // disc=2, args: label_len(u8) + label([u8;32])
        let label_bytes = b"bench-multisig";
        let mut args = Vec::with_capacity(1 + 32);
        args.push(label_bytes.len() as u8);
        let mut label = [0u8; 32];
        label[..label_bytes.len()].copy_from_slice(label_bytes);
        args.extend_from_slice(&label);
        Ok(BenchInstruction::new(make_ix_data(2, &args), metas).with_signer(creator))
    }
    
    pub fn build_execute_transfer_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-execute-transfer-creator");
        let signer_one = keypair_for_account("multisig-execute-transfer-signer-one");
        let signer_two = keypair_for_account("multisig-execute-transfer-signer-two");
        let (config, _) = config_address(&creator.pubkey());
        let (vault, _) = vault_address(&config);
        let recipient = keypair_for_account("multisig-execute-transfer-recipient");
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        ctx.airdrop(&recipient.pubkey(), 1)?;
        setup_multisig(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        // Deposit first so the vault has funds to transfer.
        {
            let dep_metas = vec![
                AccountMeta::new(creator.pubkey(), true),
                AccountMeta::new_readonly(config, false),
                AccountMeta::new(vault, false),
                AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
            ];
            ctx.execute_with_signers(
                make_ix_data(1, &1_000_000u64.to_le_bytes()),
                dep_metas,
                &[&creator],
            )?;
        }
    
        let metas = vec![
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(creator.pubkey(), false),
            AccountMeta::new(vault, false),
            AccountMeta::new(recipient.pubkey(), false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
            AccountMeta::new_readonly(signer_one.pubkey(), true),
            AccountMeta::new_readonly(signer_two.pubkey(), true),
        ];
    
        // disc=3, args: amount(u64 LE)
        Ok(
            BenchInstruction::new(make_ix_data(3, &500_000u64.to_le_bytes()), metas)
                .with_signers(vec![signer_one, signer_two]),
        )
    }
    
    fn setup_multisig(
        ctx: &mut BenchContext,
        creator: &Keypair,
        signers: &[&Keypair],
        threshold: u8,
    ) -> Result<()> {
        let (config, _) = config_address(&creator.pubkey());
        let mut metas = vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
        ];
        for signer in signers {
            metas.push(AccountMeta::new_readonly(signer.pubkey(), true));
        }
    
        let extra_signers = std::iter::once(creator as &dyn solana_signer::Signer)
            .chain(signers.iter().copied().map(|s| s as &dyn solana_signer::Signer))
            .collect::<Vec<_>>();
    
        ctx.execute_with_signers(make_ix_data(0, &[threshold]), metas, &extra_signers)?;
        Ok(())
    }
}

pub mod quasar {
    use {
        crate::bench::{keypair_for_account, BenchContext, BenchInstruction},
        anchor_lang::solana_program::instruction::AccountMeta,
        anyhow::Result,
        solana_keypair::Keypair,
        solana_pubkey::Pubkey,
        solana_signer::Signer,
    };
    
    // Quasar multisig program ID: 44444444444444444444444444444444444444444444
    fn program_id() -> Pubkey {
        "44444444444444444444444444444444444444444444"
            .parse()
            .unwrap()
    }
    
    fn config_address(creator: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"multisig", creator.as_ref()], &program_id())
    }
    
    fn vault_address(config: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault", config.as_ref()], &program_id())
    }
    
    // Quasar uses 1-byte discriminators + raw LE args (no borsh).
    fn make_ix_data(disc: u8, args: &[u8]) -> Vec<u8> {
        let mut data = Vec::with_capacity(1 + args.len());
        data.push(disc);
        data.extend_from_slice(args);
        data
    }
    
    // Use the SAME keypair seeds as v1/v2 for fair bump comparison.
    
    pub fn build_create_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-create-creator");
        let signer_one = keypair_for_account("multisig-create-signer-one");
        let signer_two = keypair_for_account("multisig-create-signer-two");
        let (config, _) = config_address(&creator.pubkey());
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
    
        // Quasar create accounts: creator, config, rent, system_program + remaining signers
        let metas = vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(solana_pubkey::Pubkey::from_str_const("SysvarRent111111111111111111111111111111111"), false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
            AccountMeta::new_readonly(signer_one.pubkey(), true),
            AccountMeta::new_readonly(signer_two.pubkey(), true),
        ];
    
        // disc=0, args: threshold(u8)
        Ok(BenchInstruction::new(make_ix_data(0, &[2u8]), metas)
            .with_signers(vec![creator, signer_one, signer_two]))
    }
    
    pub fn build_deposit_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-deposit-creator");
        let signer_one = keypair_for_account("multisig-deposit-signer-one");
        let signer_two = keypair_for_account("multisig-deposit-signer-two");
        let (config, _) = config_address(&creator.pubkey());
        let (vault, _) = vault_address(&config);
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        setup_multisig(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        // Quasar deposit accounts: depositor, config, vault, system_program
        let metas = vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
        ];
    
        // disc=1, args: amount(u64 LE)
        Ok(BenchInstruction::new(make_ix_data(1, &1_000_000u64.to_le_bytes()), metas)
            .with_signer(creator))
    }
    
    pub fn build_set_label_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-set-label-creator");
        let signer_one = keypair_for_account("multisig-set-label-signer-one");
        let signer_two = keypair_for_account("multisig-set-label-signer-two");
        let (config, _) = config_address(&creator.pubkey());
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        setup_multisig(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        // Quasar set_label accounts: creator, config, system_program
        let metas = vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
        ];
    
        // disc=2, args: String<32> = 4-byte LE length prefix + UTF-8 bytes
        let label = b"bench-multisig";
        let mut args = Vec::new();
        args.extend_from_slice(&(label.len() as u32).to_le_bytes());
        args.extend_from_slice(label);
        Ok(BenchInstruction::new(make_ix_data(2, &args), metas).with_signer(creator))
    }
    
    pub fn build_execute_transfer_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-execute-transfer-creator");
        let signer_one = keypair_for_account("multisig-execute-transfer-signer-one");
        let signer_two = keypair_for_account("multisig-execute-transfer-signer-two");
        let (config, _) = config_address(&creator.pubkey());
        let (vault, _) = vault_address(&config);
        let recipient = keypair_for_account("multisig-execute-transfer-recipient");
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        ctx.airdrop(&recipient.pubkey(), 1)?;
        setup_multisig(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        // Deposit first
        {
            let dep_metas = vec![
                AccountMeta::new(creator.pubkey(), true),
                AccountMeta::new_readonly(config, false),
                AccountMeta::new(vault, false),
                AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
            ];
            ctx.execute_with_signers(
                make_ix_data(1, &1_000_000u64.to_le_bytes()),
                dep_metas,
                &[&creator],
            )?;
        }
    
        // Quasar execute_transfer accounts: config, creator, vault, recipient, system_program + remaining signers
        let metas = vec![
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(creator.pubkey(), false),
            AccountMeta::new(vault, false),
            AccountMeta::new(recipient.pubkey(), false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
            AccountMeta::new_readonly(signer_one.pubkey(), true),
            AccountMeta::new_readonly(signer_two.pubkey(), true),
        ];
    
        // disc=3, args: amount(u64 LE)
        Ok(
            BenchInstruction::new(make_ix_data(3, &500_000u64.to_le_bytes()), metas)
                .with_signers(vec![signer_one, signer_two]),
        )
    }
    
    fn setup_multisig(
        ctx: &mut BenchContext,
        creator: &Keypair,
        signers: &[&Keypair],
        threshold: u8,
    ) -> Result<()> {
        let (config, _) = config_address(&creator.pubkey());
        let mut metas = vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(solana_pubkey::Pubkey::from_str_const("SysvarRent111111111111111111111111111111111"), false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
        ];
        for signer in signers {
            metas.push(AccountMeta::new_readonly(signer.pubkey(), true));
        }
    
        let extra_signers = std::iter::once(creator as &dyn solana_signer::Signer)
            .chain(signers.iter().copied().map(|s| s as &dyn solana_signer::Signer))
            .collect::<Vec<_>>();
    
        ctx.execute_with_signers(make_ix_data(0, &[threshold]), metas, &extra_signers)?;
        Ok(())
    }
}

pub mod steel {
    // Bench runner for the steel multisig program.
    //
    // Like pinocchio/quasar, the steel port uses a 1-byte instruction
    // discriminator + raw LE args. The runner logic is almost a carbon copy of
    // the pinocchio runner — only the account-order contract is shared, so we
    // reuse the same manually-constructed `AccountMeta`s.
    
    use {
        crate::bench::{keypair_for_account, BenchContext, BenchInstruction},
        anchor_lang::solana_program::instruction::AccountMeta,
        anyhow::Result,
        solana_keypair::Keypair,
        solana_pubkey::Pubkey,
        solana_signer::Signer,
    };
    
    fn program_id() -> Pubkey {
        "44444444444444444444444444444444444444444444"
            .parse()
            .unwrap()
    }
    
    fn config_address(creator: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"multisig", creator.as_ref()], &program_id())
    }
    
    fn vault_address(config: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault", config.as_ref()], &program_id())
    }
    
    fn make_ix_data(disc: u8, args: &[u8]) -> Vec<u8> {
        let mut data = Vec::with_capacity(1 + args.len());
        data.push(disc);
        data.extend_from_slice(args);
        data
    }
    
    pub fn build_create_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-create-creator");
        let signer_one = keypair_for_account("multisig-create-signer-one");
        let signer_two = keypair_for_account("multisig-create-signer-two");
        let (config, _) = config_address(&creator.pubkey());
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
    
        let metas = vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
            AccountMeta::new_readonly(signer_one.pubkey(), true),
            AccountMeta::new_readonly(signer_two.pubkey(), true),
        ];
    
        Ok(BenchInstruction::new(make_ix_data(0, &[2u8]), metas)
            .with_signers(vec![creator, signer_one, signer_two]))
    }
    
    pub fn build_deposit_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-deposit-creator");
        let signer_one = keypair_for_account("multisig-deposit-signer-one");
        let signer_two = keypair_for_account("multisig-deposit-signer-two");
        let (config, _) = config_address(&creator.pubkey());
        let (vault, _) = vault_address(&config);
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        setup_multisig(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        let metas = vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
        ];
    
        Ok(
            BenchInstruction::new(make_ix_data(1, &1_000_000u64.to_le_bytes()), metas)
                .with_signer(creator),
        )
    }
    
    pub fn build_set_label_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-set-label-creator");
        let signer_one = keypair_for_account("multisig-set-label-signer-one");
        let signer_two = keypair_for_account("multisig-set-label-signer-two");
        let (config, _) = config_address(&creator.pubkey());
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        setup_multisig(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        let metas = vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
        ];
    
        let label_bytes = b"bench-multisig";
        let mut args = Vec::with_capacity(1 + 32);
        args.push(label_bytes.len() as u8);
        let mut label = [0u8; 32];
        label[..label_bytes.len()].copy_from_slice(label_bytes);
        args.extend_from_slice(&label);
        Ok(BenchInstruction::new(make_ix_data(2, &args), metas).with_signer(creator))
    }
    
    pub fn build_execute_transfer_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let creator = keypair_for_account("multisig-execute-transfer-creator");
        let signer_one = keypair_for_account("multisig-execute-transfer-signer-one");
        let signer_two = keypair_for_account("multisig-execute-transfer-signer-two");
        let (config, _) = config_address(&creator.pubkey());
        let (vault, _) = vault_address(&config);
        let recipient = keypair_for_account("multisig-execute-transfer-recipient");
    
        ctx.airdrop(&creator.pubkey(), 1_000_000_000)?;
        ctx.airdrop(&recipient.pubkey(), 1)?;
        setup_multisig(ctx, &creator, &[&signer_one, &signer_two], 2)?;
    
        // Deposit first.
        {
            let dep_metas = vec![
                AccountMeta::new(creator.pubkey(), true),
                AccountMeta::new_readonly(config, false),
                AccountMeta::new(vault, false),
                AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
            ];
            ctx.execute_with_signers(
                make_ix_data(1, &1_000_000u64.to_le_bytes()),
                dep_metas,
                &[&creator],
            )?;
        }
    
        let metas = vec![
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(creator.pubkey(), false),
            AccountMeta::new(vault, false),
            AccountMeta::new(recipient.pubkey(), false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
            AccountMeta::new_readonly(signer_one.pubkey(), true),
            AccountMeta::new_readonly(signer_two.pubkey(), true),
        ];
    
        Ok(
            BenchInstruction::new(make_ix_data(3, &500_000u64.to_le_bytes()), metas)
                .with_signers(vec![signer_one, signer_two]),
        )
    }
    
    fn setup_multisig(
        ctx: &mut BenchContext,
        creator: &Keypair,
        signers: &[&Keypair],
        threshold: u8,
    ) -> Result<()> {
        let (config, _) = config_address(&creator.pubkey());
        let mut metas = vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
        ];
        for signer in signers {
            metas.push(AccountMeta::new_readonly(signer.pubkey(), true));
        }
    
        let extra_signers = std::iter::once(creator as &dyn solana_signer::Signer)
            .chain(signers.iter().copied().map(|s| s as &dyn solana_signer::Signer))
            .collect::<Vec<_>>();
    
        ctx.execute_with_signers(make_ix_data(0, &[threshold]), metas, &extra_signers)?;
        Ok(())
    }
}

