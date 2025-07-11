use crate::account::MIN_ACCOUNT_LAMPORTS;

pub struct Rent;

impl Rent {
    pub fn minimum_balance(_units: u64) -> u64 {
        MIN_ACCOUNT_LAMPORTS
    }
}
