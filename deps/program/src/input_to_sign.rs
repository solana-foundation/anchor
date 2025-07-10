//! Input requiring signature.
//!
//! `InputToSign` represents a transaction input
//! that needs a signature from a specific key.
#[cfg(feature = "fuzzing")]
use libfuzzer_sys::arbitrary;

use crate::pubkey::Pubkey;

/// Represents a transaction input that needs to be signed.
///
/// An `InputToSign` contains the index of the input within a transaction
/// and the public key of the signer that should sign this input.
#[derive(Clone, Default, Debug, Eq, PartialEq, Copy)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]

pub struct InputToSign {
    /// The index of the input within the transaction.
    pub index: u32,
    /// The public key of the signer that should sign this input.
    pub signer: Pubkey,
}
