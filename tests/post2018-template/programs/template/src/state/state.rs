use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct MyAccount {
    pub data: u64,
}

