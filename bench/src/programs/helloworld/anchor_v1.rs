use {
    crate::bench::{BenchContext, BenchInstruction},
    anchor_lang::{
        solana_program::system_program, InstructionData, ToAccountMetas,
    },
    anyhow::Result,
};

pub fn build_init_case(ctx: &mut BenchContext) -> Result<BenchInstruction> {
    let program_id = ctx.program_id();
    let (counter_pda, _bump) =
        solana_pubkey::Pubkey::find_program_address(&[b"counter"], &program_id);

    let metas = hello_world::accounts::Init {
        payer: ctx.payer_pubkey(),
        counter: counter_pda,
        system_program: system_program::ID,
    }
    .to_account_metas(None);

    Ok(BenchInstruction::new(
        hello_world::instruction::Init {}.data(),
        metas,
    ))
}
