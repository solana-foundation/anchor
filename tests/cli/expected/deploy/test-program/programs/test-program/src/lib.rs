use anchor_lang::prelude::*;

declare_id!("CMXKQmzJHCpviDpkQ5YcSjFq1cdoJqL2APANmuzy1NBf");

#[program]
pub mod test_program {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
