use anchor_lang::prelude::*;

declare_id!("aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x");

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
