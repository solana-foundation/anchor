#![feature(proc_macro_hygiene)]

use anchor::prelude::*;

#[program]
mod basic_0 {
   use super::*;
   pub fn initialize(ctx: Context<Initialize>) -> ProgramResult {
       Ok(())
   }
}

#[derive(Accounts)]
pub struct Initialize {}