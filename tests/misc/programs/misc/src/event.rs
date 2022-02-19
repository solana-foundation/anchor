use anchor_lang::prelude::*;

pub const MAX_EVENT_SIZE: usize = 10;
pub const MAX_EVENT_SIZE_U8: u8 = 11;
// pub const UNDERFLOW_EVENT_SIZE: i8 = -1;

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

#[event]
pub struct E5 {
    pub data: [u8; MAX_EVENT_SIZE],
}

#[event]
pub struct E6 {
    pub data: [u8; MAX_EVENT_SIZE_U8 as usize],
}

// #[event]
// pub struct E7 {
//     pub data: [u8; UNDERFLOW_EVENT_SIZE as usize],
// }
