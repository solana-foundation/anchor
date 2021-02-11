//! Misc example is a catchall program for testing unrelated features.
//! It's not too instructive/coherent by itself, so please see other examples.

#![feature(proc_macro_hygiene)]

use anchor_lang::prelude::*;

#[program]
pub mod misc {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>, udata: u128, idata: i128) -> ProgramResult {
        ctx.accounts.data.udata = udata;
        ctx.accounts.data.idata = idata;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init)]
    data: ProgramAccount<'info, Data>,
    rent: Sysvar<'info, Rent>,
}

#[account]
pub struct Data {
    udata: u128,
    idata: i128,
}
