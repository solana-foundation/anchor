//! Clock module provides time-related functionality for the program.
use borsh::{BorshDeserialize, BorshSerialize};

// Add the Clock struct definition
#[derive(Debug, Clone, Copy, Default, BorshSerialize, BorshDeserialize)]
pub struct Clock {
    pub slot: u64,
    pub epoch: u64,
    pub unix_timestamp: i64,
}
