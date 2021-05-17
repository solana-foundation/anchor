use crate::*;
use anchor_lang::prelude::AccountInfo;
use bytemuck::{cast_slice_mut, from_bytes_mut, try_cast_slice_mut, Pod, Zeroable};
use std::cell::RefMut;

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct AccKey {
    pub val: [u8; 32],
}

#[derive(Copy, Clone)]
#[repr(C)]
pub enum PriceStatus {
    Unknown,
    Trading,
    Halted,
    Auction,
}

impl Default for PriceStatus {
    fn default() -> Self {
        PriceStatus::Trading
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub enum CorpAction {
    NoCorpAct,
}

impl Default for CorpAction {
    fn default() -> Self {
        CorpAction::NoCorpAct
    }
}

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct PriceInfo {
    pub price: i64,
    pub conf: u64,
    pub status: PriceStatus,
    pub corp_act: CorpAction,
    pub pub_slot: u64,
}
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct PriceComp {
    publisher: AccKey,
    agg: PriceInfo,
    latest: PriceInfo,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub enum PriceType {
    Unknown,
    Price,
    TWAP,
    Volatility,
}

impl Default for PriceType {
    fn default() -> Self {
        PriceType::Price
    }
}

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct Price {
    pub magic: u32,       // Pyth magic number.
    pub ver: u32,         // Program version.
    pub atype: u32,       // Account type.
    pub size: u32,        // Price account size.
    pub ptype: PriceType, // Price or calculation type.
    pub expo: i32,        // Price exponent.
    pub num: u32,         // Number of component prices.
    pub unused: u32,
    pub curr_slot: u64,  // Currently accumulating price slot.
    pub valid_slot: u64, // Valid slot-time of agg price.
    pub prod: AccKey,
    pub next: AccKey,
    pub agg_pub: AccKey,
    pub agg: PriceInfo,
    pub comp: [PriceComp; 16],
}

impl Price {
    #[inline]
    pub fn load<'a>(price_feed: &'a AccountInfo) -> Result<RefMut<'a, Price>, ProgramError> {
        let account_data: RefMut<'a, [u8]>;
        let state: RefMut<'a, Self>;

        account_data = RefMut::map(price_feed.try_borrow_mut_data().unwrap(), |data| *data);

        state = RefMut::map(account_data, |data| {
            from_bytes_mut(cast_slice_mut::<u8, u8>(try_cast_slice_mut(data).unwrap()))
        });
        Ok(state)
    }
}

#[cfg(target_endian = "little")]
unsafe impl Zeroable for Price {}

#[cfg(target_endian = "little")]
unsafe impl Pod for Price {}
