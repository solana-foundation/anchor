use anchor_lang::prelude::*;

declare_id!("bbHgTM8c4goW91FVeYMUUE8bQgGaqNZLNRLaoK4HqnJ");

#[program]
pub mod another_program {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
