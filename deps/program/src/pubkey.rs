//! Public key definitions and operations for the Arch VM environment.
//!
//! This module defines the `Pubkey` type, which represents a 32-byte public key
//! used throughout the Arch system for identifying accounts, programs, and other entities.
//! It provides methods for creating, manipulating, and verifying public keys, including
//! program-derived addresses (PDAs).

use bitcode::{Decode, Encode};
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
#[cfg(feature = "fuzzing")]
use libfuzzer_sys::arbitrary;
use serde::{Deserialize, Serialize};

/// Number of bytes in a pubkey
pub const PUBKEY_BYTES: usize = 32;

/// A public key used to identify accounts, programs, and other entities in the Arch VM.
///
/// The `Pubkey` is a 32-byte value that uniquely identifies an entity within the Arch system.
/// It can be used as an account identifier, a program ID, or for other identification purposes.
/// The struct provides methods for serialization, creation, and verification of program-derived
/// addresses (PDAs).
#[repr(C)]
#[derive(
    Clone,
    Eq,
    PartialEq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Copy,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Pod,
    Zeroable,
    Encode,
    Decode,
)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct Pubkey(pub [u8; 32]);

impl Pubkey {
    pub const fn new_from_array(data: [u8; 32]) -> Self {
        Self(data)
    }

    /// Serializes the public key to a 32-byte array.
    ///
    /// # Returns
    /// A 32-byte array containing the public key bytes
    pub fn serialize(&self) -> [u8; 32] {
        self.0
    }

    /// Creates a new Pubkey from a slice of bytes.
    ///
    /// If the slice is shorter than 32 bytes, the remaining bytes will be padded with zeros.
    ///
    /// # Arguments
    /// * `data` - The byte slice to create the Pubkey from
    ///
    /// # Returns
    /// A new Pubkey instance
    pub fn from_slice(data: &[u8]) -> Self {
        let mut tmp = [0u8; 32];
        tmp[..data.len()].copy_from_slice(data);
        Self(tmp)
    }

    /// Returns the system program's public key.
    ///
    /// # Returns
    /// The system program's Pubkey
    pub const fn system_program() -> Self {
        let mut tmp = [0u8; 32];
        tmp[31] = 1;
        Self(tmp)
    }

    /// Checks if the Pubkey represents the system program.
    ///
    /// # Returns
    /// `true` if the Pubkey is the system program's Pubkey, `false` otherwise
    pub fn is_system_program(&self) -> bool {
        let mut tmp = [0u8; 32];
        tmp[31] = 1;
        self.0 == tmp
    }

    /// Creates a unique Pubkey for tests and benchmarks.
    ///
    /// This method generates a deterministic unique pubkey by incrementing an atomic counter.
    /// It is useful for creating distinct keys in test and benchmark environments.
    ///
    /// # Returns
    /// A new unique Pubkey instance
    pub fn new_unique() -> Self {
        use crate::atomic_u64::AtomicU64;
        static I: AtomicU64 = AtomicU64::new(1);

        let mut b = [0u8; 32];
        let i = I.fetch_add(1);
        // use big endian representation to ensure that recent unique pubkeys
        // are always greater than less recent unique pubkeys
        b[0..8].copy_from_slice(&i.to_be_bytes());
        Self::from(b)
    }

    /// Logs the Pubkey to the program log.
    ///
    /// This method is used within programs to output the public key to the program's log,
    /// which can be useful for debugging and monitoring program execution.
    ///
    /// # Safety
    /// This method makes a direct system call and should only be used within a program context.
    pub fn log(&self) {
        #[cfg(target_os = "solana")]
        unsafe {
            crate::syscalls::sol_log_pubkey(self.as_ref() as *const _ as *const u8);
        }
        #[cfg(not(target_os = "solana"))]
        crate::program_stubs::_sol_log_pubkey(self.as_ref() as *const _ as *const u8);
    }

    /// Checks if a public key represents a point on the secp256k1 curve.
    ///
    /// This is used in program address derivation to ensure that derived addresses
    /// cannot be used to sign transactions (as they don't map to valid private keys).
    ///
    /// # Arguments
    /// * `pubkey` - The public key bytes to check
    ///
    /// # Returns
    /// `true` if the pubkey is on the curve, `false` otherwise
    #[cfg(not(target_os = "solana"))]
    pub fn is_on_curve(pubkey: &[u8]) -> bool {
        match bitcoin::secp256k1::PublicKey::from_slice(pubkey) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Finds a valid program address and bump seed for the given seeds and program ID.
    ///
    /// This method searches for a program-derived address (PDA) by trying different bump seeds
    /// until it finds one that produces a valid PDA (one that is not on the curve).
    ///
    /// # Arguments
    /// * `seeds` - The seeds to use in the address derivation
    /// * `program_id` - The program ID to derive the address from
    ///
    /// # Returns
    /// A tuple containing the derived program address and the bump seed used
    ///
    /// # Panics
    /// Panics if no valid program address could be found with any bump seed
    pub fn find_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
        Self::try_find_program_address(seeds, program_id)
            .unwrap_or_else(|| panic!("Unable to find a viable program address bump seed"))
    }

    /// Attempts to find a valid program address and bump seed for the given seeds and program ID.
    ///
    /// Similar to `find_program_address`, but returns `None` instead of panicking if no valid
    /// address can be found.
    ///
    /// # Arguments
    /// * `seeds` - The seeds to use in the address derivation
    /// * `program_id` - The program ID to derive the address from
    ///
    /// # Returns
    /// An Option containing a tuple of the derived program address and bump seed if found,
    /// or None if no valid program address could be derived
    pub fn try_find_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> Option<(Pubkey, u8)> {
        // Perform the calculation inline, calling this from within a program is
        // not supported
        #[cfg(not(target_os = "solana"))]
        {
            let mut bump_seed = [std::u8::MAX];
            for _ in 0..std::u8::MAX {
                {
                    let mut seeds_with_bump = seeds.to_vec();
                    seeds_with_bump.push(&bump_seed);
                    match Self::create_program_address(&seeds_with_bump, program_id) {
                        Ok(address) => return Some((address, bump_seed[0])),
                        Err(ProgramError::InvalidSeeds) => (),
                        e => {
                            println!("error {:?}", e);
                            break;
                        }
                    }
                }
                bump_seed[0] -= 1;
            }
            None
        }
        // Call via a system call to perform the calculation
        #[cfg(target_os = "solana")]
        {
            let mut bytes = [0; 32];
            let mut bump_seed = std::u8::MAX;
            let result = unsafe {
                crate::syscalls::sol_try_find_program_address(
                    seeds as *const _ as *const u8,
                    seeds.len() as u64,
                    program_id as *const _ as *const u8,
                    &mut bytes as *mut _ as *mut u8,
                    &mut bump_seed as *mut _ as *mut u8,
                )
            };
            match result {
                crate::entrypoint::SUCCESS => Some((Pubkey::from(bytes), bump_seed)),
                _ => None,
            }
        }
    }

    /// Creates a program address (PDA) deterministically from a set of seeds and a program ID.
    ///
    /// Program addresses are deterministically derived from seeds and a program ID, but
    /// unlike normal public keys, they do not lie on the ed25519 curve and thus have no
    /// associated private key.
    ///
    /// # Arguments
    /// * `seeds` - The seeds to use in the address derivation, maximum of 16 seeds with
    ///   each seed having a maximum length of 32 bytes
    /// * `program_id` - The program ID to derive the address from
    ///
    /// # Returns
    /// The derived program address if successful
    ///
    /// # Errors
    /// Returns an error if:
    /// - There are more than MAX_SEEDS seeds
    /// - Any seed is longer than MAX_SEED_LEN bytes
    /// - The resulting address would lie on the ed25519 curve (invalid for a PDA)
    pub fn create_program_address(
        seeds: &[&[u8]],
        program_id: &Pubkey,
    ) -> Result<Pubkey, ProgramError> {
        if seeds.len() > MAX_SEEDS {
            println!("seeds.len() {} > MAX_SEEDS {}", seeds.len(), MAX_SEEDS);
            return Err(ProgramError::MaxSeedLengthExceeded);
        }
        for seed in seeds.iter() {
            if seed.len() > MAX_SEED_LEN {
                println!("seed.len() {} > MAX_SEED_LEN {}", seed.len(), MAX_SEED_LEN);
                return Err(ProgramError::MaxSeedLengthExceeded);
            }
        }

        // Perform the calculation inline, calling this from within a program is
        // not supported
        #[cfg(not(target_os = "solana"))]
        {
            let mut hash = vec![];
            for seed in seeds.iter() {
                hash.extend_from_slice(seed);
            }
            hash.extend_from_slice(program_id.as_ref());
            let hash = hex::decode(sha256::digest(&hash))?;

            if Self::is_on_curve(&hash) {
                return Err(ProgramError::InvalidSeeds);
            }

            Ok(Self::from_slice(&hash))
        }
        // Call via a system call to perform the calculation
        #[cfg(target_os = "solana")]
        {
            let mut bytes = [0; 32];
            let result = unsafe {
                crate::syscalls::sol_create_program_address(
                    seeds as *const _ as *const u8,
                    seeds.len() as u64,
                    program_id as *const _ as *const u8,
                    &mut bytes as *mut _ as *mut u8,
                )
            };
            match result {
                crate::entrypoint::SUCCESS => Ok(Self::from_slice(&bytes)),
                _ => Err(result.into()),
            }
        }
    }

    /// Creates a derived address based on a base public key, string seed and owner program id.
    ///
    /// Mirrors the behaviour of Solana's `Pubkey::create_with_seed` helper and is
    /// required by higher-level crates (e.g. Anchor) when working with
    /// SystemProgram instructions such as `CreateAccountWithSeed`.
    ///
    /// The resulting address is simply `sha256(base || seed || owner)` and **can** be
    /// on-curve – it is *not* restricted to PDAs.
    ///
    /// # Arguments
    /// * `base`  – Base public key that must sign any transaction creating the account
    /// * `seed`  – Arbitrary UTF-8 seed text (≤ `MAX_SEED_LEN` bytes)
    /// * `owner` – Program id that will own the created account
    ///
    /// # Errors
    /// * [`ProgramError::MaxSeedLengthExceeded`] – if the seed is longer than
    ///   `MAX_SEED_LEN`
    pub fn create_with_seed(
        base: &Pubkey,
        seed: &str,
        owner: &Pubkey,
    ) -> Result<Pubkey, ProgramError> {
        if seed.len() > MAX_SEED_LEN {
            return Err(ProgramError::MaxSeedLengthExceeded);
        }

        // Perform the calculation directly when not running inside the Solana VM
        #[cfg(not(target_os = "solana"))]
        {
            let mut data = Vec::with_capacity(32 + seed.len() + 32);
            data.extend_from_slice(base.as_ref());
            data.extend_from_slice(seed.as_bytes());
            data.extend_from_slice(owner.as_ref());

            // sha256::digest returns a hex string – decode back into raw bytes
            let hash = hex::decode(sha256::digest(&data))?;
            return Ok(Pubkey::from_slice(&hash));
        }

        // Inside the BPF program we delegate to the corresponding syscall
        #[cfg(target_os = "solana")]
        {
            unimplemented!()
        }
    }
}

/// Maximum number of seeds allowed in PDA derivation
pub const MAX_SEEDS: usize = 16;
/// Maximum length in bytes for each seed used in PDA derivation
pub const MAX_SEED_LEN: usize = 32;

impl std::fmt::LowerHex for Pubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let ser = self.serialize();
        for ch in &ser[..] {
            write!(f, "{:02x}", *ch)?;
        }
        Ok(())
    }
}

use core::fmt;

use crate::program_error::ProgramError;

/// TODO:
///  Change this in future according to the correct base implementation
impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl fmt::Debug for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl AsMut<[u8]> for Pubkey {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0[..]
    }
}

impl From<[u8; 32]> for Pubkey {
    fn from(value: [u8; 32]) -> Self {
        Pubkey(value)
    }
}

impl std::str::FromStr for Pubkey {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Decode the provided base58 string into raw bytes.
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|_| "Invalid base58 string for Pubkey")?;
        if bytes.len() != 32 {
            return Err("Invalid length for Pubkey (expected 32 bytes)");
        }
        Ok(Pubkey::from_slice(&bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pubkey::Pubkey;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn fuzz_serialize_deserialize_pubkey(data in any::<[u8; 32]>()) {
            let pubkey = Pubkey::from(data);
            let serialized = pubkey.serialize();
            let deserialized = Pubkey::from_slice(&serialized);
            assert_eq!(pubkey, deserialized);
        }
    }

    #[test]
    fn test_create_program_address() {
        let program_id = Pubkey::new_unique();

        // Test empty seeds
        let result = Pubkey::create_program_address(&[], &program_id);
        assert!(result.is_ok());

        // Test with valid seeds
        let seed1 = b"hello";
        let seed2 = b"world";
        let result = Pubkey::create_program_address(&[seed1, seed2], &program_id);
        assert!(result.is_ok());

        // Test exceeding MAX_SEEDS
        let too_many_seeds = vec![&[0u8; 1][..]; MAX_SEEDS + 1];
        let result = Pubkey::create_program_address(&too_many_seeds[..], &program_id);
        assert_eq!(result.unwrap_err(), ProgramError::MaxSeedLengthExceeded);

        // Test exceeding MAX_SEED_LEN
        let long_seed = &[0u8; MAX_SEED_LEN + 1];
        let result = Pubkey::create_program_address(&[long_seed], &program_id);
        assert_eq!(result.unwrap_err(), ProgramError::MaxSeedLengthExceeded);
    }

    #[test]
    fn test_find_program_address() {
        let program_id = Pubkey::new_unique();
        let seed1: &[u8] = b"hello";

        // Test basic functionality
        let (address, bump) = Pubkey::find_program_address(&[seed1], &program_id);
        assert!(bump <= std::u8::MAX);

        // Verify that the found address is valid
        let mut seeds_with_bump = vec![seed1];
        let bump_array = [bump];
        seeds_with_bump.push(&bump_array);
        let created_address =
            Pubkey::create_program_address(&seeds_with_bump, &program_id).unwrap();
        assert_eq!(address, created_address);
    }

    #[test]
    fn test_try_find_program_address() {
        let program_id = Pubkey::new_unique();
        let seed1: &[u8] = b"hello";

        // Test basic functionality
        let result = Pubkey::try_find_program_address(&[seed1], &program_id);
        assert!(result.is_some());

        let (address, bump) = result.unwrap();
        assert!(bump <= std::u8::MAX);

        // Verify that the found address is valid
        let mut seeds_with_bump = vec![seed1];
        let bump_array = [bump];
        seeds_with_bump.push(&bump_array);
        let created_address =
            Pubkey::create_program_address(&seeds_with_bump, &program_id).unwrap();
        assert_eq!(address, created_address);
    }
}
