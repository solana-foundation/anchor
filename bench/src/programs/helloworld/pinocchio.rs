use {
    crate::bench::{BenchContext, BenchInstruction},
    anchor_lang::solana_program::{instruction::AccountMeta, system_program},
    anyhow::Result,
};

pub fn build_init_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
    let program_id = ctx.program_id();
    let (counter_pda, _bump) =
        solana_pubkey::Pubkey::find_program_address(&[b"counter"], &program_id);

    let metas = vec![
        AccountMeta::new(ctx.payer_pubkey(), true),
        AccountMeta::new(counter_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    // Program derives the PDA + bump on-chain (same as v2/quasar/v1/steel),
    // so we pass empty instruction data.
    Ok(BenchInstruction::new(vec![], metas))
}
