#![allow(unexpected_cfgs)]
/*!
# Arch Program
A Rust library for building programs that run inside the Arch Virtual Machine. This crate
provides core functionality for creating instructions, managing accounts, handling program
errors, and interacting with the Arch runtime environment.
## Features
- Bitcoin transaction and UTXO management
- Account data manipulation and ownership verification
- System instruction creation and processing
- Program error handling
- Logging utilities
- Cryptographic operations including secp256k1 signature recovery
- Memory management for on-chain programs
## Usage
Add this crate to your `Cargo.toml`:
```toml
[dependencies]
arch_program = "0.4.0"
```
Then import the modules you need in your code:
```rust
use arch_program::account::AccountInfo;
use arch_program::pubkey::Pubkey;
use arch_program::instruction::Instruction;
// ... other imports as needed
```
*/

pub use bitcoin;

/// Account management and ownership verification
pub mod account;
/// Atomic operations for u64 values
pub mod atomic_u64;
pub mod bpf_loader;
/// Time-related functionality for on-chain programs
pub mod clock;
pub mod compiled_keys;
/// Compute budget instruction definitions and processing
pub mod compute_budget;
/// Utilities for debugging account data
pub mod debug_account_data;
/// Error handling for decoding operations
pub mod decode_error;
/// Program entrypoint definitions and processing
pub mod entrypoint;
/// Helper functions for common operations
pub mod helper;
/// Bitcoin transaction input signing utilities
pub mod input_to_sign;
/// Instruction definitions and processing
pub mod instruction;
pub mod keccak;
pub mod loader_instruction;
/// Logging functionality for on-chain programs
pub mod log;
/// Message format and processing utilities
pub mod message;
pub mod native_loader;
/// Program runtime interfaces and state management
pub mod program;
/// Error types for program operations
pub mod program_error;
/// Memory management for program execution
pub mod program_memory;
/// Optional value representation for programs
pub mod program_option;
/// Data serialization and deserialization for on-chain storage
pub mod program_pack;
/// Stub implementations for program interfaces
pub mod program_stubs;
pub mod program_utils;
/// Public key definitions and operations
pub mod pubkey;
/// Sanitization trait and error types for validating over-the-wire messages
pub mod sanitize;
/// Sanitized transaction processing
pub mod sanitized;
/// Secp256k1 signature recovery utilities
pub mod sol_secp256k1_recover;
/// Stable memory layout implementations
pub mod stable_layout;
pub mod stake;
/// System call interfaces for interacting with the runtime
pub mod syscalls;
/// System instruction definitions and creation
pub mod system_instruction;
pub mod system_program;
/// Bitcoin transaction signing utilities
pub mod transaction_to_sign;
/// Bitcoin UTXO (Unspent Transaction Output) management
pub mod utxo;
pub mod vote;

#[macro_use]
extern crate serde_derive;

/// Rune management
pub mod rune;

/// Maximum size of a Bitcoin transaction in bytes
pub const MAX_BTC_TX_SIZE: usize = 3976;

/// Maximum size of a Bitcoin rune output in bytes
pub const MAX_BTC_RUNE_OUTPUT_SIZE: usize = 2048;

pub mod builtin {
    use super::*;
    use crate::pubkey::Pubkey;

    pub const BUILTIN_PROGRAMS_ID: &[Pubkey] = &[
        native_loader::NATIVE_LOADER_ID,
        system_program::SYSTEM_PROGRAM_ID,
        bpf_loader::BPF_LOADER_ID,
    ];
}
