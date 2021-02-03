//! This example demonstrates how custom errors and associated error messsages
//! can be defined and transparently propagated to clients.

#![feature(proc_macro_hygiene)]

use anchor_lang::prelude::*;

#[program]
mod errors {
    use super::*;
    pub fn hello(_ctx: Context<Hello>) -> Result<()> {
        Err(MyError::Hello.into())
    }

    pub fn hello_no_msg(_ctx: Context<Hello>) -> Result<()> {
        Err(MyError::HelloNoMsg.into())
    }

    pub fn hello_next(_ctx: Context<Hello>) -> Result<()> {
        Err(MyError::HelloNext.into())
    }
}

#[derive(Accounts)]
pub struct Hello {}

#[error]
pub enum MyError {
    #[msg("This is an error message clients will automatically display")]
    Hello,
    HelloNoMsg = 123,
    HelloNext,
}
