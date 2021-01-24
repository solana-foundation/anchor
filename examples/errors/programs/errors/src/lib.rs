#![feature(proc_macro_hygiene)]

use anchor_lang::prelude::*;

#[program]
mod errors {
    use super::*;
    pub fn hello(ctx: Context<Hello>) -> Result<()> {
        Err(MyError::Hello.into())
    }

    pub fn hello_no_msg(ctx: Context<HelloNoMsg>) -> Result<()> {
        Err(MyError::HelloNoMsg.into())
    }

    pub fn hello_next(ctx: Context<HelloNext>) -> Result<()> {
        Err(MyError::HelloNext.into())
    }
}

#[derive(Accounts)]
pub struct Hello {}

#[derive(Accounts)]
pub struct HelloNoMsg {}

#[derive(Accounts)]
pub struct HelloNext {}

#[error]
pub enum MyError {
    #[msg("This is an error message clients will automatically display")]
    Hello,
    HelloNoMsg = 123,
    HelloNext,
}
