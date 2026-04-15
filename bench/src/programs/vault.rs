pub mod anchor_v2 {
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
}

pub mod quasar {
    use {
        crate::bench::{keypair_for_account, BenchContext, BenchInstruction},
        anchor_lang::solana_program::{instruction::AccountMeta, system_program},
        anyhow::Result,
        solana_account::Account as SolanaAccount,
        solana_pubkey::Pubkey,
        solana_signer::Signer,
    };
    
    // Quasar vault program ID: 33333333333333333333333333333333333333333333
    // (matches `declare_id!` in the `quasar-vault` example copied from
    // `the quasar vault example sourcelib.rs`.)
    fn program_id() -> Pubkey {
        "33333333333333333333333333333333333333333333"
            .parse()
            .unwrap()
    }
    
    fn vault_address(user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault", user.as_ref()], &program_id())
    }
    
    // Quasar uses 1-byte discriminators + raw LE args (no borsh).
    fn make_ix_data(disc: u8, args: &[u8]) -> Vec<u8> {
        let mut data = Vec::with_capacity(1 + args.len());
        data.push(disc);
        data.extend_from_slice(args);
        data
    }
    
    pub fn build_deposit_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let user = keypair_for_account("vault-deposit-user");
        let (vault, _) = vault_address(&user.pubkey());
    
        ctx.airdrop(&user.pubkey(), 1_000_000_000)?;
    
        // Quasar Deposit accounts: user, vault, system_program.
        let metas = vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ];
    
        // disc=0, args: amount(u64 LE)
        Ok(
            BenchInstruction::new(make_ix_data(0, &1_000_000u64.to_le_bytes()), metas)
                .with_signer(user),
        )
    }
    
    pub fn build_withdraw_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
        let user = keypair_for_account("vault-withdraw-user");
        let (vault, _) = vault_address(&user.pubkey());
    
        ctx.airdrop(&user.pubkey(), 1_000_000_000)?;
        // Pre-fund the vault PDA as **program-owned**, mirroring
        // quasar's own `test_withdraw` setup in
        // `the quasar vault example sourcetests.rs`. Quasar's `deposit`
        // uses `system::Transfer` which leaves the vault system-owned,
        // so the subsequent `withdraw` (direct lamport arithmetic) would
        // fail with `ExternalAccountLamportSpend`. Their test harness
        // sidesteps this by creating the vault directly as an owned
        // account, and we replicate that here for a fair comparison.
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
    
        // Quasar Withdraw accounts: user, vault (no system_program — withdraw
        // mutates lamports directly via arithmetic, no CPI).
        let metas = vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new(vault, false),
        ];
    
        // disc=1, args: amount(u64 LE)
        Ok(
            BenchInstruction::new(make_ix_data(1, &1_000_000u64.to_le_bytes()), metas)
                .with_signer(user),
        )
    }
}

