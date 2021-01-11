#![feature(proc_macro_hygiene)]

use anchor::prelude::*;

#[program]
mod sysvars {
    use super::*;
    pub fn sysvars(_ctx: Context<Sysvars>) -> ProgramResult {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Sysvars {
    pub clock: Clock,
    pub rent: Rent,
    pub stake_history: StakeHistory,
}
