use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, Same};

use super::{MempoolEntry, Txid};

/// Maximum capacity within Solana's 5MB account size limit
const MAX_ENTRIES: usize = 25_600;
const BUCKET_COUNT: usize = 256;
const BUCKET_SIZE: usize = MAX_ENTRIES / BUCKET_COUNT;

#[serde_as]
#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[repr(C)]
pub struct MempoolEntries {
    #[serde_as(as = "[Same; BUCKET_COUNT]")]
    pub counts: [u16; BUCKET_COUNT],
    #[serde_as(as = "[[Same; BUCKET_SIZE]; BUCKET_COUNT]")]
    pub entries: [[MempoolEntry; BUCKET_SIZE]; BUCKET_COUNT],
}

impl MempoolEntries {
    fn bucket_for(&self, txid: &Txid) -> usize {
        // Use first 2 bytes for better distribution
        ((txid[0] as usize) << 8 | (txid[1] as usize)) % BUCKET_COUNT
    }

    pub fn get(&self, txid: &Txid) -> Option<&MempoolEntry> {
        let bucket = self.bucket_for(txid);
        let count = self.counts[bucket] as usize;

        // Linear search within the bucket
        for i in 0..count {
            if &self.entries[bucket][i].txid == txid {
                return Some(&self.entries[bucket][i]);
            }
        }
        None
    }

    pub fn iter(&self) -> impl Iterator<Item = &MempoolEntry> {
        (0..BUCKET_COUNT).flat_map(move |bucket| {
            let count = self.counts[bucket] as usize;
            (0..count).map(move |i| &self.entries[bucket][i])
        })
    }

    pub fn keys(&self) -> impl Iterator<Item = &Txid> {
        (0..BUCKET_COUNT).flat_map(move |bucket| {
            let count = self.counts[bucket] as usize;
            (0..count).map(move |i| &self.entries[bucket][i].txid)
        })
    }

    pub fn contains_key(&self, txid: &Txid) -> bool {
        self.get(txid).is_some()
    }

    pub fn extend(&mut self, other: Self) {
        for entry in other.iter() {
            self.add_entry(*entry);
        }
    }

    /// Adds a new entry. If there's already an entry with this txid, it's replaced.
    /// If there's no space left in the bucket, the oldest entry is overwritten.
    pub fn add_entry(&mut self, entry: MempoolEntry) {
        let txid = entry.txid;
        let bucket = self.bucket_for(&txid);
        let count = self.counts[bucket] as usize;

        // Check if this txid already exists in the bucket
        for i in 0..count {
            if &self.entries[bucket][i].txid == &txid {
                // If entry is different, replace it
                self.entries[bucket][i] = entry;
                return;
            }
        }

        // If we have space in this bucket, add it
        if count < BUCKET_SIZE {
            self.entries[bucket][count] = entry;
            self.counts[bucket] += 1;
        } else {
            // No space in bucket - implement replacement (FIFO)
            // Shift all entries to make room at the end
            for i in 0..BUCKET_SIZE - 1 {
                self.entries[bucket][i] = self.entries[bucket][i + 1];
            }

            // Insert at the last position
            let last = BUCKET_SIZE - 1;
            self.entries[bucket][last] = entry;
        }
    }

    /// Removes an entry with the specified txid
    pub fn remove_entry(&mut self, txid: Txid) {
        let bucket = self.bucket_for(&txid);
        let count = self.counts[bucket] as usize;

        for i in 0..count {
            if &self.entries[bucket][i].txid == &txid {
                // Shift all following elements down
                for j in i..count - 1 {
                    self.entries[bucket][j] = self.entries[bucket][j + 1];
                }
                self.counts[bucket] -= 1;
                return;
            }
        }
    }

    /// Updates an existing entry or adds a new one
    pub fn update_entry(&mut self, entry: MempoolEntry) {
        self.add_entry(entry);
    }

    /// Deserializes a MempoolEntries struct from a slice of bytes
    pub fn deserialize_from_slice(data: &[u8]) -> std::io::Result<HashMap<Txid, MempoolEntry>> {
        let mut result = HashMap::new();

        // Read counts array
        if data.len() < std::mem::size_of::<[u16; BUCKET_COUNT]>() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Data too small",
            ));
        }

        let mut pos = 0;

        // Process each bucket without creating the entire structure
        for bucket in 0..BUCKET_COUNT {
            // Read count
            let count_bytes = [data[pos], data[pos + 1]];
            let count = u16::from_le_bytes(count_bytes) as usize;
            pos += 2;

            // Skip to the entries section for this bucket
            let entries_offset = std::mem::size_of::<[u16; BUCKET_COUNT]>()
                + bucket * BUCKET_SIZE * std::mem::size_of::<MempoolEntry>();

            // Process each entry in this bucket
            for i in 0..count {
                if entries_offset
                    + i * std::mem::size_of::<MempoolEntry>()
                    + std::mem::size_of::<MempoolEntry>()
                    > data.len()
                {
                    break;
                }

                let entry_pos = entries_offset + i * std::mem::size_of::<MempoolEntry>();
                let entry: MempoolEntry =
                    borsh::BorshDeserialize::deserialize(&mut &data[entry_pos..])?;
                result.insert(entry.txid, entry);
            }
        }

        Ok(result)
    }
}
