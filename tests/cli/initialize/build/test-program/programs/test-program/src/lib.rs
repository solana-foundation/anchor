use anchor_lang::prelude::*;

declare_id!("F5mz67v9rW6z73nLHK29d45Jh6PMGiGPz71autX9Hp4D");

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
