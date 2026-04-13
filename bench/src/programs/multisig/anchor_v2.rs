use {
    crate::bench::{keypair_for_account, BenchContext, BenchInstruction},
    anchor_lang::solana_program::instruction::AccountMeta,
    anyhow::Result,
    anchor_lang_v2::{Address, InstructionData, ToAccountMetas},
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
};

/// Convert an anchor-lang-v2 `Address` to a solana-pubkey `Pubkey`.
fn addr_to_pubkey(addr: &Address) -> Pubkey {
    Pubkey::new_from_array(addr.to_bytes())
}

/// Convert a `Pubkey` to an anchor-lang-v2 `Address`.
fn pubkey_to_addr(pubkey: &Pubkey) -> Address {
    Address::new_from_array(pubkey.to_bytes())
}

/// Convert v2 AccountMetas to solana AccountMetas.
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
