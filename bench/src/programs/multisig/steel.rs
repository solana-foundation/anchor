//! Bench runner for the steel multisig program.
//!
//! Like pinocchio/quasar, the steel port uses a 1-byte instruction
//! discriminator + raw LE args. The runner logic is almost a carbon copy of
//! the pinocchio runner — only the account-order contract is shared, so we
//! reuse the same manually-constructed `AccountMeta`s.

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
