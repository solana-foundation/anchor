use anchor_lang::prelude::*;

pub const MAX_EVENT_SIZE: usize = 10;
pub const MAX_EVENT_SIZE_U8: u8 = 11;

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

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
pub struct TestStruct {
    pub data1: u8,
    pub data2: u16,
    pub data3: u32,
    pub data4: u64,
}

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
pub enum TestEnum {
    First,
    Second { x: u64, y: u64 },
    TupleTest(u8, u8, u16, u16),
    TupleStructTest(TestStruct),
}

#[event]
pub struct E7 {
    pub data: TestEnum,
}
