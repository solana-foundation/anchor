use {
    crate::bench::{keypair_for_account, BenchContext, BenchInstruction},
    anchor_lang::solana_program::{instruction::AccountMeta, system_program},
    anchor_lang_v2::InstructionData,
    anyhow::Result,
    solana_account::Account as SolanaAccount,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
};

fn program_id() -> Pubkey {
    "33333333333333333333333333333333333333333333"
        .parse()
        .unwrap()
}

fn vault_address(user: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", user.as_ref()], &program_id())
}

pub fn build_deposit_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
    let user = keypair_for_account("vault-deposit-user");
    let (vault, _) = vault_address(&user.pubkey());

    ctx.airdrop(&user.pubkey(), 1_000_000_000)?;

    let metas = vec![
        AccountMeta::new(user.pubkey(), true),
        AccountMeta::new(vault, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(BenchInstruction::new(
        vault_v2::instruction::Deposit { amount: 1_000_000 }.data(),
        metas,
    )
    .with_signer(user))
}

pub fn build_withdraw_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
    let user = keypair_for_account("vault-withdraw-user");
    let (vault, _) = vault_address(&user.pubkey());

    ctx.airdrop(&user.pubkey(), 1_000_000_000)?;
    // Same pre-funding rationale as the quasar variant: withdraw needs
    // the vault to be program-owned so direct lamport arithmetic is
    // permitted, but `deposit` via system::Transfer leaves it system-
    // owned. Matches quasar's own `test_withdraw` setup.
    ctx.svm_mut().set_account(
        vault,
        SolanaAccount {
            lamports: 1_000_000_000,
            data: vec![],
            owner: program_id(),
            executable: false,
            rent_epoch: 0,
        },
    ).map_err(|e| anyhow::anyhow!("set_account failed: {e:?}"))?;

    // Withdraw takes user + vault, no system_program (mutates lamports
    // directly via arithmetic).
    let metas = vec![
        AccountMeta::new(user.pubkey(), true),
        AccountMeta::new(vault, false),
    ];

    Ok(BenchInstruction::new(
        vault_v2::instruction::Withdraw { amount: 1_000_000 }.data(),
        metas,
    )
    .with_signer(user))
}
