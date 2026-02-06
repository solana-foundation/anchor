use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111111");

#[program]
pub mod raw_instruction_fail {
    use super::*;

    // This should fail compilation - &[u8] argument without #[raw] attribute
    pub fn should_fail(ctx: Context<ShouldFail>, data: &[u8]) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct ShouldFail<'info> {
    pub signer: Signer<'info>,
}
