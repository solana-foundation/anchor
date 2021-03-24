//! Misc example is a catchall program for testing unrelated features.
//! It's not too instructive/coherent by itself, so please see other examples.

#![feature(proc_macro_hygiene)]

use anchor_lang::prelude::*;

#[program]
pub mod misc {
    use super::*;

    pub const SIZE: u64 = 99;

    #[state(SIZE)]
    pub struct MyState {
        pub v: Vec<u8>,
    }

    impl MyState {
        pub fn new(_ctx: Context<Ctor>) -> Result<Self, ProgramError> {
            Ok(Self { v: vec![] })
        }
    }

    pub fn initialize(ctx: Context<Initialize>, udata: u128, idata: i128) -> ProgramResult {
        ctx.accounts.data.udata = udata;
        ctx.accounts.data.idata = idata;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Ctor {}

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
