//! # Saturn Account Shards
//!
//! A high-performance library for managing sharded account state in Bitcoin-based blockchain programs.
//! This crate provides ergonomic tools for working with distributed UTXO sets
//! across multiple shards while maintaining atomicity and consistency.
//!
//! ## Overview
//!
//! This library is designed for applications that need to manage large amounts of Bitcoin UTXOs
//! and Rune tokens across multiple shards for scalability. It provides:
//!
//! - **Type-safe shard management** with compile-time state tracking
//! - **Efficient UTXO distribution** across shards
//! - **Atomic transaction building** with proper fee calculation
//! - **Rune token support** for managing fungible tokens on Bitcoin
//! - **Zero-copy serialization** for high-performance applications
//!
//! ## Quick start
//!
//! 1. **Add the dependency** to your `Cargo.toml` (defaults enable `runes` and
//!    `utxo-consolidation`):
//!
//! ```toml
//! saturn-account-shards = "0.1"
//! ```
//!
//! 2. **Create a `ShardSet`, select shards, call helpers**:
//!
//! ```rust,no_run
//! use saturn_account_shards::{ShardSet, StateShard};
//! # use saturn_bitcoin_transactions::utxo_info::{UtxoInfo, SingleRuneSet};
//! # #[derive(Default, Clone)]
//! # struct DummyShard;
//! # impl StateShard<UtxoInfo<SingleRuneSet>, SingleRuneSet> for DummyShard {
//! #   fn btc_utxos(&self) -> &[UtxoInfo<SingleRuneSet>] { &[] }
//! #   fn btc_utxos_mut(&mut self) -> &mut [UtxoInfo<SingleRuneSet>] { &mut [] }
//! #   fn btc_utxos_retain(&mut self, _: &mut dyn FnMut(&UtxoInfo<SingleRuneSet>) -> bool) {}
//! #   fn add_btc_utxo(&mut self, _: UtxoInfo<SingleRuneSet>) -> Option<usize> { None }
//! #   fn btc_utxos_len(&self) -> usize { 0 }
//! #   fn btc_utxos_max_len(&self) -> usize { 0 }
//! #   fn rune_utxo(&self) -> Option<&UtxoInfo<SingleRuneSet>> { None }
//! #   fn rune_utxo_mut(&mut self) -> Option<&mut UtxoInfo<SingleRuneSet>> { None }
//! #   fn clear_rune_utxo(&mut self) {}
//! #   fn set_rune_utxo(&mut self, _: UtxoInfo<SingleRuneSet>) {}
//! # }
//! # let mut shards_storage = [DummyShard::default(), DummyShard::default()];
//! # let mut loaders: Vec<&mut DummyShard> = shards_storage.iter_mut().collect();
//!
//! // Build an *unselected* set and then pick the shards we need.
//! let shard_set = ShardSet::<SingleRuneSet, UtxoInfo<SingleRuneSet>, DummyShard, 2>::from_loaders(&mut loaders);
//! let mut selected = shard_set.select_with([0, 1]).unwrap();
//!
//! // Call any of the high-level helpers, e.g. redistribute BTC back to the shards.
//! // selected.redistribute_remaining_btc_to_shards(...);
//! ```
//!
//! ## Features
//!
//! | Feature flag | Purpose | Default |
//! |--------------|---------|---------|
//! | `runes` | Support for the Bitcoin **Runes** protocol (adds Rune-specific helpers) | ✅ |
//! | `utxo-consolidation` | Convenience helpers to combine many small UTXOs into fewer large ones | ✅ |
//! | `serde` | Enable `serde::{Serialize, Deserialize}` implementations where possible | ❌ |
//!
//! All items that require a feature are tagged on docs.rs via
//! **`cfg(feature = "…")`** in their rust-doc header.
//!
//! ## Glossary
//!
//! | Term | Meaning |
//! |------|---------|
//! | **Shard** | A single on-chain account that stores a portion of the global program state. |
//! | **Pool** | A collection of shards that together form one logical program state. |
//! | **UTXO** | *Unspent Transaction Output*: holds satoshis (and optionally Rune amounts) on the Bitcoin chain. |
//! | **Rune** | A fungible token defined by the Runes protocol and transported inside a Bitcoin UTXO. |
//!
//! ## Key Components
//!
//! ### ShardSet
//!
//! The central abstraction is [`ShardSet`], which provides a type-safe wrapper around
//! a collection of shards. It uses the typestate pattern to ensure proper usage:
//!
//! ```rust
//! # use saturn_account_shards::{ShardSet, StateShard};
//! # use saturn_bitcoin_transactions::utxo_info::{UtxoInfo, SingleRuneSet};
//! # #[derive(Default, Clone)]
//! # struct DummyShard;
//! # impl StateShard<UtxoInfo<SingleRuneSet>, SingleRuneSet> for DummyShard {
//! #     fn btc_utxos(&self) -> &[UtxoInfo<SingleRuneSet>] { &[] }
//! #     fn btc_utxos_mut(&mut self) -> &mut [UtxoInfo<SingleRuneSet>] { &mut [] }
//! #     fn btc_utxos_retain(&mut self, _: &mut dyn FnMut(&UtxoInfo<SingleRuneSet>) -> bool) {}
//! #     fn add_btc_utxo(&mut self, _: UtxoInfo<SingleRuneSet>) -> Option<usize> { None }
//! #     fn btc_utxos_len(&self) -> usize { 0 }
//! #     fn btc_utxos_max_len(&self) -> usize { 0 }
//! #     fn rune_utxo(&self) -> Option<&UtxoInfo<SingleRuneSet>> { None }
//! #     fn rune_utxo_mut(&mut self) -> Option<&mut UtxoInfo<SingleRuneSet>> { None }
//! #     fn clear_rune_utxo(&mut self) {}
//! #     fn set_rune_utxo(&mut self, _: UtxoInfo<SingleRuneSet>) {}
//! # }
//! # let mut shard1 = DummyShard::default();
//! # let mut shard2 = DummyShard::default();
//! # let mut shards = [&mut shard1, &mut shard2];
//! // Create an unselected shard set
//! let shard_set = ShardSet::<SingleRuneSet, UtxoInfo<SingleRuneSet>, DummyShard, 2>::new(&mut shards);
//!
//! // Select shards to work with
//! let selected = shard_set.select_with([0, 1]).unwrap();
//!
//! // Now high-level operations are available
//! // selected.redistribute_remaining_btc_to_shards(...);
//! ```
//!
//! ### StateShard Trait
//!
//! All shards must implement the [`StateShard`] trait, which provides a unified interface
//! for managing both Bitcoin UTXOs and Rune tokens:
//!
//! ```rust
//! # use saturn_account_shards::{StateShard};
//! # use saturn_bitcoin_transactions::utxo_info::{UtxoInfo, SingleRuneSet};
//! # use saturn_collections::generic::fixed_set::FixedCapacitySet;
//! #
//! struct MyShard {
//!     btc_utxos: Vec<UtxoInfo<SingleRuneSet>>,
//!     rune_utxo: Option<UtxoInfo<SingleRuneSet>>,
//! }
//!
//! impl StateShard<UtxoInfo<SingleRuneSet>, SingleRuneSet> for MyShard {
//!     fn btc_utxos(&self) -> &[UtxoInfo<SingleRuneSet>] {
//!         &self.btc_utxos
//!     }
//!
//!     fn btc_utxos_mut(&mut self) -> &mut [UtxoInfo<SingleRuneSet>] {
//!         &mut self.btc_utxos
//!     }
//!
//!     // ... implement other required methods
//! #     fn btc_utxos_retain(&mut self, _: &mut dyn FnMut(&UtxoInfo<SingleRuneSet>) -> bool) {}
//! #     fn add_btc_utxo(&mut self, _: UtxoInfo<SingleRuneSet>) -> Option<usize> { None }
//! #     fn btc_utxos_len(&self) -> usize { 0 }
//! #     fn btc_utxos_max_len(&self) -> usize { 0 }
//! #     fn rune_utxo(&self) -> Option<&UtxoInfo<SingleRuneSet>> { None }
//! #     fn rune_utxo_mut(&mut self) -> Option<&mut UtxoInfo<SingleRuneSet>> { None }
//! #     fn clear_rune_utxo(&mut self) {}
//! #     fn set_rune_utxo(&mut self, _: UtxoInfo<SingleRuneSet>) {}
//! # }
//! ```
//!
//! ## Common Operations
//!
//! ### Selecting Shards
//!
//! ```rust
//! # use saturn_account_shards::{ShardSet, StateShard};
//! # use saturn_bitcoin_transactions::utxo_info::{UtxoInfo, SingleRuneSet};
//! # #[derive(Default, Clone)]
//! # struct DummyShard;
//! # impl StateShard<UtxoInfo<SingleRuneSet>, SingleRuneSet> for DummyShard {
//! #     fn btc_utxos(&self) -> &[UtxoInfo<SingleRuneSet>] { &[] }
//! #     fn btc_utxos_mut(&mut self) -> &mut [UtxoInfo<SingleRuneSet>] { &mut [] }
//! #     fn btc_utxos_retain(&mut self, _: &mut dyn FnMut(&UtxoInfo<SingleRuneSet>) -> bool) {}
//! #     fn add_btc_utxo(&mut self, _: UtxoInfo<SingleRuneSet>) -> Option<usize> { None }
//! #     fn btc_utxos_len(&self) -> usize { 0 }
//! #     fn btc_utxos_max_len(&self) -> usize { 0 }
//! #     fn rune_utxo(&self) -> Option<&UtxoInfo<SingleRuneSet>> { None }
//! #     fn rune_utxo_mut(&mut self) -> Option<&mut UtxoInfo<SingleRuneSet>> { None }
//! #     fn clear_rune_utxo(&mut self) {}
//! #     fn set_rune_utxo(&mut self, _: UtxoInfo<SingleRuneSet>) {}
//! # }
//! # let mut shards: Vec<DummyShard> = vec![DummyShard::default(); 5];
//! # let mut shard_refs: Vec<&mut DummyShard> = shards.iter_mut().collect();
//! let shard_set = ShardSet::<SingleRuneSet, UtxoInfo<SingleRuneSet>, DummyShard, 5>::new(&mut shard_refs);
//!
//! // Select specific shards by index
//! let selected = shard_set.select_with([0, 2, 4]).unwrap();
//!
//! // Select the shard with minimum BTC
//! // let selected = shard_set.select_min_by(|s| s.total_btc()).unwrap();
//!
//! // Select shards that meet a condition
//! // let selected = shard_set.select_multiple_by(|s| s.btc_utxos_len() > 0).unwrap();
//! ```
//!
//! ### Redistributing Liquidity
//!
//! ```rust,no_run
//! # use saturn_account_shards::{ShardSet, StateShard};
//! # use saturn_bitcoin_transactions::{TransactionBuilder, fee_rate::FeeRate};
//! # use saturn_bitcoin_transactions::utxo_info::{UtxoInfo, SingleRuneSet};
//! # use std::str::FromStr;
//! # use bitcoin::ScriptBuf;
//! # #[derive(Default, Clone)]
//! # struct DummyShard;
//! # impl StateShard<UtxoInfo<SingleRuneSet>, SingleRuneSet> for DummyShard {
//! #     fn btc_utxos(&self) -> &[UtxoInfo<SingleRuneSet>] { &[] }
//! #     fn btc_utxos_mut(&mut self) -> &mut [UtxoInfo<SingleRuneSet>] { &mut [] }
//! #     fn btc_utxos_retain(&mut self, _: &mut dyn FnMut(&UtxoInfo<SingleRuneSet>) -> bool) {}
//! #     fn add_btc_utxo(&mut self, _: UtxoInfo<SingleRuneSet>) -> Option<usize> { None }
//! #     fn btc_utxos_len(&self) -> usize { 0 }
//! #     fn btc_utxos_max_len(&self) -> usize { 0 }
//! #     fn rune_utxo(&self) -> Option<&UtxoInfo<SingleRuneSet>> { None }
//! #     fn rune_utxo_mut(&mut self) -> Option<&mut UtxoInfo<SingleRuneSet>> { None }
//! #     fn clear_rune_utxo(&mut self) {}
//! #     fn set_rune_utxo(&mut self, _: UtxoInfo<SingleRuneSet>) {}
//! # }
//! # let mut shards: Vec<DummyShard> = vec![DummyShard::default(); 3];
//! # let mut shard_refs: Vec<&mut DummyShard> = shards.iter_mut().collect();
//! # let shard_set = ShardSet::<SingleRuneSet, UtxoInfo<SingleRuneSet>, DummyShard, 3>::new(&mut shard_refs);
//! # let mut selected = shard_set.select_with([0, 1, 2]).unwrap();
//! # let mut tx_builder = TransactionBuilder::<10, 3, SingleRuneSet>::new();
//! # let program_script = ScriptBuf::new();
//! # let fee_rate = FeeRate::from_str("1.0").unwrap();
//! # let removed_from_shards = 1000u64;
//! // Redistribute remaining BTC evenly across selected shards
//! let redistributed = selected.redistribute_remaining_btc_to_shards(
//!     &mut tx_builder,
//!     removed_from_shards,
//!     program_script,
//!     &fee_rate,
//! ).unwrap();
//! ```

#[cfg(test)]
mod tests;

mod error;
mod prelude;
mod shard;
mod shard_handle;
mod shard_indices;
mod shard_set;
mod split;
mod update;

pub use error::{Result, StateShardError};
pub use prelude::{SelectedShards, Shards};
pub use shard::{AccountUtxos, StateShard};
pub use shard_handle::ShardHandle;
pub use shard_indices::IntoShardIndices;
pub use shard_set::ShardSet;
pub use shard_set::{Selected, Unselected};

pub use saturn_collections::{declare_fixed_array, declare_fixed_option, declare_fixed_set};
