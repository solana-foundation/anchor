use anchor_lang::prelude::*;

#[event]
pub struct E1 {
    pub data: u32,
}

#[event]
pub struct E2 {
    pub data: u32,
}

#[event]
pub struct E3 {
    pub data: u32,
}

#[event]
pub struct E4 {
    pub data: Pubkey,
}
