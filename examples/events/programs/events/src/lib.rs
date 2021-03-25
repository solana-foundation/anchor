#![feature(proc_macro_hygiene)]

use anchor_lang::prelude::*;

#[program]
pub mod events {
    use super::*;
    pub fn initialize(_ctx: Context<Initialize>) -> ProgramResult {
        emit!(MyEvent {
            data: 5,
            label: "hello".to_string(),
        });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

#[event]
pub struct MyEvent {
    pub data: u64,
    #[index]
    pub label: String,
}
