//! Transaction to sign representation for serialization and deserialization.
//!
//! This module provides the `TransactionToSign` struct which represents a transaction
//! along with the inputs that need to be signed.

use crate::input_to_sign::InputToSign;
use crate::pubkey::Pubkey;

/// Represents a transaction that needs to be signed with associated inputs.
///
/// This struct holds the raw transaction bytes and a list of inputs that need to be
/// signed, each with their own index and signer public key.
#[repr(C)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TransactionToSign<'a> {
    /// The raw transaction bytes to be signed.
    pub tx_bytes: &'a [u8],
    /// List of inputs within the transaction that need signatures.
    pub inputs_to_sign: &'a [InputToSign],
}

impl<'a> TransactionToSign<'a> {
    /// Serializes the `TransactionToSign` into a byte vector.
    ///
    /// The serialized format is:
    /// - 4 bytes: length of tx_bytes (u32, little endian)
    /// - N bytes: tx_bytes content
    /// - 4 bytes: number of inputs to sign (u32, little endian)
    /// - For each input:
    ///   - 4 bytes: input index (u32, little endian)
    ///   - 32 bytes: signer public key
    ///
    /// # Returns
    ///
    /// A vector of bytes containing the serialized transaction.
    pub fn serialise(&self) -> Vec<u8> {
        let mut serialized = vec![];

        serialized.extend_from_slice(&(self.tx_bytes.len() as u32).to_le_bytes());
        serialized.extend_from_slice(self.tx_bytes);
        serialized.extend_from_slice(&(self.inputs_to_sign.len() as u32).to_le_bytes());
        for input_to_sign in self.inputs_to_sign.iter() {
            serialized.extend_from_slice(&input_to_sign.index.to_le_bytes());
            serialized.extend_from_slice(&input_to_sign.signer.serialize());
        }

        serialized
    }

    /// Serializes a Transaction and inputs_to_sign directly into a single buffer
    /// without intermediate allocations
    ///
    /// # Parameters
    ///
    /// * `tx` - A reference to the Bitcoin transaction to serialize.
    /// * `inputs_to_sign` - A slice of `InputToSign` structs representing the inputs to sign.
    ///
    /// # Returns
    ///
    /// A vector of bytes containing the serialized transaction.
    pub fn serialise_with_tx(tx: &bitcoin::Transaction, inputs_to_sign: &[InputToSign]) -> Vec<u8> {
        use bitcoin::consensus::Encodable;

        // Estimate initial capacity to minimize reallocations
        let inputs_count = inputs_to_sign.len();
        let estimated_tx_size = 250; // Conservative estimate for average tx size
        let total_estimated_size = 4 + estimated_tx_size + 4 + (4 + 32) * inputs_count;
        let mut serialized = Vec::with_capacity(total_estimated_size);

        // Reserve 4 bytes for tx length (we'll write this after we know the actual length)
        let tx_len_pos = serialized.len();
        serialized.extend_from_slice(&[0u8; 4]); // Placeholder for tx length

        // Serialize transaction directly into the buffer
        let tx_start = serialized.len();
        tx.consensus_encode(&mut serialized)
            .expect("Transaction encoding should not fail");
        let tx_end = serialized.len();
        let tx_len = tx_end - tx_start;

        // Write the actual tx length back to the reserved space
        serialized[tx_len_pos..tx_len_pos + 4].copy_from_slice(&(tx_len as u32).to_le_bytes());

        // Serialize inputs_to_sign
        serialized.extend_from_slice(&(inputs_count as u32).to_le_bytes());
        for input_to_sign in inputs_to_sign.iter() {
            serialized.extend_from_slice(&input_to_sign.index.to_le_bytes());
            serialized.extend_from_slice(&input_to_sign.signer.serialize());
        }

        serialized
    }

    /// Deserializes a byte slice into a `TransactionToSign`.
    ///
    /// # Parameters
    ///
    /// * `data` - A byte slice containing the serialized transaction.
    ///
    /// # Returns
    ///
    /// A new `TransactionToSign` instance.
    ///
    /// # Panics
    ///
    /// This function will panic if the input data is malformed or doesn't contain
    /// enough bytes for the expected format.
    pub fn from_slice(data: &'a [u8]) -> Self {
        let mut size = 0;

        let tx_bytes_len = u32::from_le_bytes(data[size..size + 4].try_into().unwrap()) as usize;
        size += 4;

        let tx_bytes = &data[size..(size + tx_bytes_len)];
        size += tx_bytes_len;

        let inputs_to_sign_len =
            u32::from_le_bytes(data[size..size + 4].try_into().unwrap()) as usize;
        size += 4;

        let mut inputs_to_sign = Vec::with_capacity(inputs_to_sign_len);

        for _ in 0..inputs_to_sign_len {
            let index = u32::from_le_bytes(data[size..size + 4].try_into().unwrap());
            size += 4;

            let signer = Pubkey::from_slice(&data[size..size + 32]);
            size += 32;

            inputs_to_sign.push(InputToSign { index, signer });
        }

        TransactionToSign {
            tx_bytes,
            inputs_to_sign: inputs_to_sign.leak(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        input_to_sign::InputToSign, pubkey::Pubkey, transaction_to_sign::TransactionToSign,
    };
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn fuzz_serialize_deserialize_transaction_to_sign(
            tx_bytes in prop::collection::vec(any::<u8>(), 0..64),
            input_indices in prop::collection::vec(any::<u32>(), 0..10),
            input_pubkeys in prop::collection::vec(any::<[u8; 32]>(), 0..10)
        ) {
            let inputs_to_sign: Vec<InputToSign> = input_indices.into_iter()
                .zip(input_pubkeys.into_iter())
                .map(|(index, pubkey_bytes)| {
                    InputToSign {
                        index,
                        signer: Pubkey::from(pubkey_bytes),
                    }
                })
                .collect();

            let transaction = TransactionToSign {
                tx_bytes: &tx_bytes,
                inputs_to_sign: &inputs_to_sign,
            };

            let serialized = transaction.serialise();
            let deserialized = TransactionToSign::from_slice(&serialized);

            assert_eq!(transaction.tx_bytes, deserialized.tx_bytes);
            assert_eq!(transaction.inputs_to_sign, deserialized.inputs_to_sign);
        }
    }

    #[test]
    fn test_serialise_with_tx_consistency() {
        use bitcoin::consensus::serialize;
        use bitcoin::{Amount, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Witness};

        // Create a simple test transaction
        let tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint::default(),
                script_sig: ScriptBuf::new(),
                sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(5000),
                script_pubkey: ScriptBuf::new(),
            }],
        };

        let inputs_to_sign = vec![
            InputToSign {
                index: 0,
                signer: Pubkey::from([1u8; 32]),
            },
            InputToSign {
                index: 1,
                signer: Pubkey::from([2u8; 32]),
            },
        ];

        // Method 1: Original approach (serialize -> TransactionToSign -> serialise)
        let tx_bytes = serialize(&tx);
        let transaction_to_sign = TransactionToSign {
            tx_bytes: &tx_bytes,
            inputs_to_sign: &inputs_to_sign,
        };
        let serialized1 = transaction_to_sign.serialise();

        // Method 2: New approach (serialise_with_tx)
        let serialized2 = TransactionToSign::serialise_with_tx(&tx, &inputs_to_sign);

        // They should produce identical results
        assert_eq!(serialized1, serialized2);

        // Verify both can be deserialized correctly
        let deserialized1 = TransactionToSign::from_slice(&serialized1);
        let deserialized2 = TransactionToSign::from_slice(&serialized2);

        assert_eq!(deserialized1.tx_bytes, deserialized2.tx_bytes);
        assert_eq!(deserialized1.inputs_to_sign, deserialized2.inputs_to_sign);
        assert_eq!(deserialized1.tx_bytes, &tx_bytes);
        assert_eq!(deserialized1.inputs_to_sign, &inputs_to_sign);
    }
}
