use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct RuneId {
    pub block: u64,
    pub tx: u32,
    _padding: [u8; 4],
}

impl RuneId {
    pub const BTC: Self = RuneId {
        block: 0,
        tx: 0,
        _padding: [0; 4],
    };

    pub fn new(block: u64, tx: u32) -> Self {
        Self {
            block,
            tx,
            _padding: [0; 4],
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}", self.block, self.tx)
    }

    /// Returns token bytes as a fixed-size array without heap allocation
    pub fn to_bytes(&self) -> [u8; 12] {
        let mut result = [0u8; 12];
        result[0..8].copy_from_slice(&self.block.to_le_bytes());
        result[8..12].copy_from_slice(&self.tx.to_le_bytes());
        result
    }
}

impl BorshSerialize for RuneId {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        borsh::BorshSerialize::serialize(&self.block, writer)?;
        borsh::BorshSerialize::serialize(&self.tx, writer)?;
        Ok(())
    }
}

impl BorshDeserialize for RuneId {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let block = <u64 as borsh::BorshDeserialize>::deserialize(buf)?;
        let tx = <u32 as borsh::BorshDeserialize>::deserialize(buf)?;
        Ok(RuneId::new(block, tx))
    }

    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let block = u64::deserialize_reader(reader)?;
        let tx = u32::deserialize_reader(reader)?;
        Ok(RuneId::new(block, tx))
    }
}

impl FromStr for RuneId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split(':').collect::<Vec<&str>>();

        if parts.len() != 2 {
            return Err("Invalid format: expected 'block:tx'".to_string());
        }

        let block = parts[0]
            .parse::<u64>()
            .map_err(|_| "Invalid block number")?;
        let tx = parts[1]
            .parse::<u32>()
            .map_err(|_| "Invalid transaction number")?;
        Ok(RuneId::new(block, tx))
    }
}

impl Serialize for RuneId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RuneId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        let rune_id = RuneId::from_str(&s).map_err(serde::de::Error::custom)?;
        Ok(rune_id)
    }
}

#[derive(
    Debug, Copy, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, Pod, Zeroable,
)]
#[repr(C)]
pub struct RuneAmount {
    pub id: RuneId,
    #[serde(
        serialize_with = "serialize_u128",
        deserialize_with = "deserialize_u128"
    )]
    pub amount: u128,
}

impl RuneAmount {
    pub fn zero() -> Self {
        Self {
            id: RuneId::default(),
            amount: 0,
        }
    }
}

impl Default for RuneAmount {
    fn default() -> Self {
        Self::zero()
    }
}

impl PartialOrd for RuneAmount {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let same_id = self.id == other.id;
        let amt_ord = self.amount.cmp(&other.amount);

        match (same_id, amt_ord) {
            (false, _) => None,
            (true, ord) => Some(ord),
        }
    }
}

impl PartialEq<RuneId> for RuneAmount {
    fn eq(&self, other: &RuneId) -> bool {
        self.id == *other
    }
}

impl PartialEq<RuneAmount> for RuneAmount {
    fn eq(&self, other: &RuneAmount) -> bool {
        self.id == other.id
    }
}

fn serialize_u128<S>(num: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&num.to_string())
}

fn deserialize_u128<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = <String as serde::Deserialize>::deserialize(deserializer)?;
    s.parse::<u128>().map_err(serde::de::Error::custom)
}
