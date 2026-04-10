//! Well-known program marker types for use with `Program<T>`.

use pinocchio::address::Address;
use crate::v2::Id;

/// Marker type for the System Program.
pub struct System;

impl Id for System {
    fn id() -> Address {
        Address::from_str_const("11111111111111111111111111111111")
    }
}
