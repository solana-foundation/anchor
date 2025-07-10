//! Secp256k1 elliptic curve cryptography recovery operations.
//!
//! This module provides functionality for recovering public keys from
//! signatures and message hashes using the secp256k1 elliptic curve.
//! It's primarily used for signature verification in blockchain applications.

use thiserror::Error;

/// Length of a secp256k1 signature in bytes (64 bytes without recovery id)
pub const SECP256K1_SIGNATURE_LENGTH: usize = 64;
/// Length of an uncompressed secp256k1 public key in bytes
pub const SECP256K1_PUBLIC_KEY_LENGTH: usize = 64;
/// Length of a message hash in bytes (32 bytes for SHA-256)
pub const HASH_BYTES: usize = 32;
/// Success return code for secp256k1 operations
pub const SUCCESS: u64 = 0;

/// An uncompressed secp256k1 public key.
///
/// The public key is represented as 64 bytes: the x-coordinate followed by the y-coordinate.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Secp256k1Pubkey(pub [u8; SECP256K1_PUBLIC_KEY_LENGTH]);

impl Secp256k1Pubkey {
    /// Creates a new Secp256k1Pubkey from a byte slice.
    ///
    /// # Arguments
    ///
    /// * `pubkey_vec` - A slice containing the public key bytes
    ///
    /// # Panics
    ///
    /// Panics if the provided slice is not exactly [`SECP256K1_PUBLIC_KEY_LENGTH`] bytes.
    pub fn new(pubkey_vec: &[u8]) -> Self {
        Self(
            <[u8; SECP256K1_PUBLIC_KEY_LENGTH]>::try_from(<&[u8]>::clone(&pubkey_vec))
                .expect("Slice must be the same length as a Pubkey"),
        )
    }

    /// Returns the public key as a fixed-size byte array.
    ///
    /// # Returns
    ///
    /// A 64-byte array containing the uncompressed public key
    pub fn to_bytes(self) -> [u8; 64] {
        self.0
    }
}

/// Errors that can occur during secp256k1 public key recovery.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Secp256k1RecoverError {
    /// The provided hash is invalid or has incorrect length
    #[error("The hash provided to a secp256k1_recover is invalid")]
    InvalidHash,
    /// The recovery ID is outside the valid range (should be 0, 1, 2, or 3)
    #[error("The recovery_id provided to a secp256k1_recover is invalid")]
    InvalidRecoveryId,
    /// The signature is invalid or has incorrect length
    #[error("The signature provided to a secp256k1_recover is invalid")]
    InvalidSignature,
}

/// Converts a numeric error code to a Secp256k1RecoverError
impl From<u64> for Secp256k1RecoverError {
    fn from(v: u64) -> Secp256k1RecoverError {
        match v {
            1 => Secp256k1RecoverError::InvalidHash,
            2 => Secp256k1RecoverError::InvalidRecoveryId,
            3 => Secp256k1RecoverError::InvalidSignature,
            _ => panic!("Unsupported Secp256k1RecoverError"),
        }
    }
}

/// Converts a Secp256k1RecoverError to a numeric error code
impl From<Secp256k1RecoverError> for u64 {
    fn from(v: Secp256k1RecoverError) -> u64 {
        match v {
            Secp256k1RecoverError::InvalidHash => 1,
            Secp256k1RecoverError::InvalidRecoveryId => 2,
            Secp256k1RecoverError::InvalidSignature => 3,
        }
    }
}

/// Recovers a public key from a signature, message hash, and recovery ID.
///
/// This function uses the secp256k1 elliptic curve to recover the public key
/// that was used to create the signature for the given message hash.
///
/// # Arguments
///
/// * `hash` - The 32-byte message hash that was signed
/// * `recovery_id` - The recovery ID (0, 1, 2, or 3) that identifies which of the possible
///                  public keys was used to create the signature
/// * `signature` - The 64-byte signature (r and s components without recovery ID)
///
/// # Returns
///
/// * `Ok(Secp256k1Pubkey)` - The recovered public key if successful
/// * `Err(Secp256k1RecoverError)` - The specific error that occurred during recovery
///
/// # Platform-specific Behavior
///
/// This function has different implementations based on the target platform:
/// - On Solana, it uses the `sol_secp256k1_recover` syscall
/// - On other platforms, it uses a program stub implementation
pub fn secp256k1_recover(
    hash: &[u8],
    recovery_id: u8,
    signature: &[u8],
) -> Result<Secp256k1Pubkey, Secp256k1RecoverError> {
    let mut pubkey_buffer = [0u8; SECP256K1_PUBLIC_KEY_LENGTH];

    #[cfg(target_os = "solana")]
    {
        let result = unsafe {
            crate::syscalls::sol_secp256k1_recover(
                hash.as_ptr(),
                recovery_id as u64,
                signature.as_ptr(),
                pubkey_buffer.as_mut_ptr(),
            )
        };

        match result {
            crate::entrypoint::SUCCESS => Ok(Secp256k1Pubkey::new(&pubkey_buffer)),
            _ => Err(result.into()),
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        let result = crate::program_stubs::sol_secp256k1_recover(
            hash.as_ptr(),
            recovery_id as u64,
            signature.as_ptr(),
            pubkey_buffer.as_mut_ptr(),
        );
        match result {
            crate::entrypoint::SUCCESS => Ok(Secp256k1Pubkey::new(&pubkey_buffer)),
            _ => Err(result.into()),
        }
    }
}
