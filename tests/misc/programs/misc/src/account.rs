use anchor_lang::prelude::*;

#[account]
pub struct Data {
    pub udata: u128,
    pub idata: i128,
}

#[account]
#[derive(Default)]
pub struct DataU16 {
    pub data: u16,
}

#[account]
#[derive(Default)]
pub struct DataI8 {
    pub data: i8,
}

#[account]
pub struct DataI16 {
    pub data: i16,
}

#[account(zero_copy)]
#[derive(Default)]
pub struct DataZeroCopy {
    pub data: u16,
    pub bump: u8,
}

#[account]
#[derive(Default)]
pub struct DataWithFilter {
    pub authority: Pubkey,
    pub filterable: Pubkey,
}

#[account]
pub struct DataMultidimensionalArray {
    pub data: [[u8; 10]; 10],
}
