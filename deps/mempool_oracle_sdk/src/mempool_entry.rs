use std::fmt;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use super::Txid;

/// MempoolEntry structure for the smart contract
///
/// `total_fee` and `total_vsize` represent values for this entry and all of
/// its ancestors
#[derive(
    Default,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    Eq,
    Copy,
    Hash,
)]
#[serde(rename_all = "camelCase")]
pub struct MempoolEntry {
    pub txid: Txid,       // 16 bytes
    pub total_fee: u64,   // 8 bytes
    pub total_vsize: u64, // 8 bytes
    pub descendants: u16, // 2 bytes
    pub ancestors: u16,   // 2 bytes
                          // 20 bytes + 16 bytes = 36
                          // 10MB => 10_000_000 / 36 ~= 300k
                          // Return:  20 bytes + 1 byte (option) = 21
                          // 1KB => 1024 / 21 = 48
}

impl fmt::Display for MempoolEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "txid: {}, total_fee: {}, total_vsize: {}, descendants: {}, ancestors: {}",
            self.txid, self.total_fee, self.total_vsize, self.descendants, self.ancestors
        )
    }
}

impl fmt::Debug for MempoolEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Default, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub struct ReturnedMempoolEntry {
    pub total_fee: u64,
    pub total_vsize: u64,
    pub descendants: u16,
    pub ancestors: u16,
}

impl From<MempoolEntry> for ReturnedMempoolEntry {
    fn from(entry: MempoolEntry) -> Self {
        Self {
            total_fee: entry.total_fee,
            total_vsize: entry.total_vsize,
            descendants: entry.descendants,
            ancestors: entry.ancestors,
        }
    }
}
