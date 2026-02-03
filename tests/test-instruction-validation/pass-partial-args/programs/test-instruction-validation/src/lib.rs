#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod test_instruction_validation {
    use super::*;

    // Test: Handler has 4 args (a, b, c, d) but #[instruction] declares c and d (non-sequential)
    // This should compile successfully, proving the optimization works for multiple partial args
    pub fn partial_args(
        _ctx: Context<PartialArgs>,
        a: u64,  // Handler arg 0 - will be skipped
        b: u32,  // Handler arg 1 - declared in #[instruction], will be used
        c: u64,  // Handler arg 2 - declared in #[instruction], will be used
        d: u8,   // Handler arg 3 - declared in #[instruction], will be used
    ) -> Result<()> {
        msg!("a: {}, b: {}, c: {}, d: {}", a, b, c, d);
        Ok(())
    }

    #[derive(Accounts)]
    #[instruction(b: u32, c: u64, d: u8)] 
    pub struct PartialArgs<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
    }
}
