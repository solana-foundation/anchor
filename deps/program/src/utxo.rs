//! Bitcoin UTXO (Unspent Transaction Output) management and processing.
//!
//! This module provides utilities for working with Bitcoin UTXOs in the Arch VM environment.
//! It includes functionality for serializing, deserializing, and managing UTXO metadata,
//! which combines a transaction ID (txid) and output index (vout) to uniquely identify
//! a specific Bitcoin UTXO.

use bitcoin::hashes::Hash;
#[cfg(feature = "fuzzing")]
use libfuzzer_sys::arbitrary;

use bitcode::{Decode, Encode};
use borsh::{BorshDeserialize, BorshSerialize};

/// Represents metadata for a Bitcoin UTXO (Unspent Transaction Output).
///
/// A UTXO is uniquely identified by a transaction ID (txid) and an output index (vout).
/// This struct stores these values in a compact 36-byte array format:
/// - First 32 bytes: transaction ID (txid)
/// - Last 4 bytes: output index (vout) in little-endian format
#[derive(Clone, Debug, PartialEq, Copy, Hash, Eq, Encode, Decode)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
#[repr(C)]
pub struct UtxoMeta([u8; 36]);

impl UtxoMeta {
    /// Creates a new UtxoMeta from a raw transaction ID and output index.
    ///
    /// # Arguments
    /// * `txid` - The 32-byte transaction ID
    /// * `vout` - The output index within the transaction
    ///
    /// # Returns
    /// A new UtxoMeta instance
    pub fn from(txid: [u8; 32], vout: u32) -> Self {
        let mut data: [u8; 36] = [0; 36];
        data[..32].copy_from_slice(&txid);
        data[32..].copy_from_slice(&vout.to_le_bytes());
        Self(data)
    }

    /// Creates a new UtxoMeta from a Bitcoin OutPoint structure.
    ///
    /// # Arguments
    /// * `txid` - The Bitcoin transaction ID
    /// * `vout` - The output index within the transaction
    ///
    /// # Returns
    /// A new UtxoMeta instance
    pub fn from_outpoint(txid: Txid, vout: u32) -> Self {
        let mut data: [u8; 36] = [0; 36];
        data[..32].copy_from_slice(
            &bitcoin::consensus::serialize(&txid)
                .into_iter()
                .rev()
                .collect::<Vec<u8>>(),
        );
        data[32..].copy_from_slice(&vout.to_le_bytes());
        Self(data)
    }

    /// Converts this UtxoMeta to a Bitcoin OutPoint structure.
    ///
    /// # Returns
    /// A Bitcoin OutPoint representing this UTXO
    pub fn to_outpoint(&self) -> OutPoint {
        OutPoint {
            txid: self.to_txid(),
            vout: self.vout(),
        }
    }

    /// Creates a new UtxoMeta from a slice of bytes.
    ///
    /// # Arguments
    /// * `data` - A byte slice containing at least 36 bytes of UTXO data
    ///
    /// # Returns
    /// A new UtxoMeta instance
    ///
    /// # Panics
    /// Panics if the slice is shorter than 36 bytes
    pub fn from_slice(data: &[u8]) -> Self {
        Self(data[..36].try_into().expect("utxo meta is 36 bytes long"))
    }

    /// Returns a reference to the transaction ID bytes.
    ///
    /// # Returns
    /// A slice containing the 32-byte transaction ID
    pub fn txid(&self) -> &[u8] {
        &self.0[..32]
    }

    /// Returns the txid bytes in big-endian format (display format).
    ///
    /// **Use this when:**
    /// - Comparing with other UtxoMeta instances
    /// - Calling Arch VM helpers (e.g., get_output_value_from_tx)
    /// - Creating new UtxoMeta instances from existing ones
    /// - Displaying txids to users (matches block explorer format)
    /// - Serializing for APIs or external systems
    ///
    /// This is the same as txid() but with clearer semantics about byte order.
    pub fn txid_big_endian(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&self.0[..32]);
        bytes
    }

    /// Returns the txid bytes in little-endian format (Bitcoin internal format).
    ///
    /// **Use this when:**
    /// - Working with Bitcoin protocol functions that expect wire format
    /// - Interacting with low-level Bitcoin operations
    ///
    /// This reverses the stored bytes to match Bitcoin's internal representation.
    pub fn txid_little_endian(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&self.0[..32]);
        bytes.reverse(); // Convert from stored big-endian to little-endian
        bytes
    }

    /// Returns the txid as a Bitcoin Txid type.
    ///
    /// **Use this when:**
    /// - Converting back to OutPoint
    /// - Working with Bitcoin library functions that expect Txid type
    /// - Need to leverage Txid's built-in methods
    ///
    /// This properly handles the byte order conversion from UtxoMeta's big-endian
    /// storage to Bitcoin's internal little-endian representation.
    pub fn to_txid(&self) -> Txid {
        let little_endian_bytes = self.txid_little_endian();
        Txid::from_byte_array(little_endian_bytes)
    }

    /// Returns a mutable reference to the transaction ID bytes.
    ///
    /// # Returns
    /// A mutable slice containing the 32-byte transaction ID
    pub fn txid_mut(&mut self) -> &mut [u8] {
        &mut self.0[..32]
    }

    /// Returns a mutable reference to the vout (output index) bytes.
    ///
    /// # Returns
    /// A mutable slice containing the 4-byte vout in little-endian format
    pub fn vout_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0[32..]
    }

    /// Returns the output index (vout) as a u32 value.
    ///
    /// # Returns
    /// The output index as a u32 value
    ///
    /// # Panics
    /// Panics if the vout bytes cannot be converted to a u32 (should not happen in practice)
    pub fn vout(&self) -> u32 {
        u32::from_le_bytes(self.0[32..].try_into().expect("utxo meta unreachable"))
    }

    /// Serializes the UtxoMeta into its raw 36-byte array representation.
    ///
    /// # Returns
    /// A 36-byte array containing the serialized UTXO metadata
    pub fn serialize(&self) -> [u8; 36] {
        self.0
    }
}

impl Default for UtxoMeta {
    fn default() -> Self {
        UtxoMeta([0; 36])
    }
}

#[test]
fn test_outpoint() {
    assert_eq!(
        OutPoint::new(
            Txid::from_str("c5cc9251192330191366016c8dab0f67dc345bd024a206c313dbf26db0a66bb1")
                .unwrap(),
            0
        ),
        UtxoMeta::from(
            hex::decode("c5cc9251192330191366016c8dab0f67dc345bd024a206c313dbf26db0a66bb1")
                .unwrap()
                .try_into()
                .unwrap(),
            0
        )
        .to_outpoint()
    );
}

use core::fmt;
use std::io::{Read, Result, Write};
use std::str::FromStr;

use bitcoin::OutPoint;
use bitcoin::Txid;

/// TODO:
///  Change this in future according to the correct base implementation
impl fmt::Display for UtxoMeta {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// Allows accessing the UtxoMeta's internal bytes as a slice.
impl AsRef<[u8]> for UtxoMeta {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

/// Allows accessing the UtxoMeta's internal bytes as a mutable slice.
impl AsMut<[u8]> for UtxoMeta {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0[..]
    }
}

/// Creates a UtxoMeta from a 36-byte array.
impl From<[u8; 36]> for UtxoMeta {
    fn from(value: [u8; 36]) -> Self {
        UtxoMeta(value)
    }
}

/// Implements Borsh serialization for UtxoMeta.
impl BorshSerialize for UtxoMeta {
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.0.serialize(writer)
    }
}

/// Implements Borsh deserialization for UtxoMeta.
impl BorshDeserialize for UtxoMeta {
    #[inline]
    fn deserialize_reader<R: Read>(reader: &mut R) -> Result<Self> {
        if let Some(vec_bytes) = u8::vec_from_reader(36, reader)? {
            Ok(UtxoMeta::from_slice(&vec_bytes))
        } else {
            // TODO(16): return capacity allocation when we can safely do that.
            let mut result = Vec::with_capacity(36);
            for _ in 0..36 {
                result.push(u8::deserialize_reader(reader)?);
            }
            Ok(UtxoMeta::from_slice(&result))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::utxo::UtxoMeta;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn fuzz_serialize_deserialize_utxo_meta(txid in any::<[u8; 32]>(), vout in any::<u32>()) {
            let original = UtxoMeta::from(txid, vout);
            let serialized = borsh::to_vec(&original).unwrap();
            let deserialized: UtxoMeta = borsh::from_slice(&serialized).unwrap();
            assert_eq!(original, deserialized);
        }
    }
}
