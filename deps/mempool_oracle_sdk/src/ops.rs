use borsh::{BorshDeserialize, BorshSerialize};
use hex;
use serde::{Deserialize, Serialize};

use super::{MempoolEntry, Txid};

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
pub enum UpdateMempoolEntriesInstruction {
    ModifyEntries(Vec<UpdateMempoolEntriesOp>),
    GetEntries(Vec<[u8; 32]>),
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
pub struct UpdateMempoolEntries(Vec<UpdateMempoolEntriesOp>);

impl UpdateMempoolEntries {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn get_entry(&self, index: usize) -> Option<&UpdateMempoolEntriesOp> {
        self.0.get(index)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push(&mut self, op: UpdateMempoolEntriesOp) {
        self.0.push(op);
    }

    pub fn iter(&self) -> impl Iterator<Item = &UpdateMempoolEntriesOp> {
        self.0.iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = UpdateMempoolEntriesOp> {
        self.0.into_iter()
    }

    pub fn split_by_size(
        &self,
        max_size: usize,
    ) -> Result<Vec<UpdateMempoolEntries>, std::io::Error> {
        let mut result = Vec::new();
        let mut current = UpdateMempoolEntries::new();

        for op in &self.0 {
            // Try adding the operation to the current batch
            current.push(op.clone());

            let serialized = borsh::to_vec(&current.0)?;

            // Check if we've exceeded the max size
            if serialized.len() > max_size {
                // Remove the last added operation
                current.0.pop();

                // Only add non-empty batches to the result
                if !current.0.is_empty() {
                    result.push(current);
                    current = UpdateMempoolEntries::new();

                    // Add the operation to the new batch
                    current.push(op.clone());
                }
            }
        }

        // Add the final batch if it's not empty
        if !current.0.is_empty() {
            result.push(current);
        }

        Ok(result)
    }
}

impl Into<UpdateMempoolEntriesInstruction> for UpdateMempoolEntries {
    fn into(self) -> UpdateMempoolEntriesInstruction {
        UpdateMempoolEntriesInstruction::ModifyEntries(self.0)
    }
}

impl Into<UpdateMempoolEntriesInstruction> for &UpdateMempoolEntries {
    fn into(self) -> UpdateMempoolEntriesInstruction {
        UpdateMempoolEntriesInstruction::ModifyEntries(self.0.clone())
    }
}

impl FromIterator<UpdateMempoolEntriesOp> for UpdateMempoolEntries {
    fn from_iter<I: IntoIterator<Item = UpdateMempoolEntriesOp>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl IntoIterator for UpdateMempoolEntries {
    type Item = UpdateMempoolEntriesOp;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a UpdateMempoolEntries {
    type Item = &'a UpdateMempoolEntriesOp;
    type IntoIter = std::slice::Iter<'a, UpdateMempoolEntriesOp>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(
    Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, PartialEq, Eq, Hash,
)]
#[serde(tag = "operation", content = "data")]
pub enum UpdateMempoolEntriesOp {
    #[serde(rename = "add_entry")]
    #[serde(serialize_with = "serialize_entry")]
    AddEntry(MempoolEntry),
    #[serde(rename = "delete_entry")]
    #[serde(serialize_with = "serialize_txid")]
    DeleteEntry(Txid),
    #[serde(rename = "update_entry")]
    #[serde(serialize_with = "serialize_entry")]
    UpdateEntry(MempoolEntry),
}

fn serialize_entry<S>(entry: &MempoolEntry, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeStruct;

    let mut state = serializer.serialize_struct("EntryWithTxid", 2)?;
    state.serialize_field("txid", &entry.txid)?;
    state.serialize_field("entry", entry)?;
    state.end()
}

fn serialize_txid<S>(txid: &Txid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeStruct;

    let mut state = serializer.serialize_struct("TxidOnly", 1)?;
    state.serialize_field("txid", &txid)?;
    state.end()
}

impl UpdateMempoolEntriesOp {
    pub fn txid(&self) -> &Txid {
        match self {
            UpdateMempoolEntriesOp::AddEntry(entry) => &entry.txid,
            UpdateMempoolEntriesOp::DeleteEntry(txid) => txid,
            UpdateMempoolEntriesOp::UpdateEntry(entry) => &entry.txid,
        }
    }
}
