//! # Saturn Collections
//!
//! Generic, fixed-capacity data structures & utility macros designed for **Arch / Saturn on-chain programs**.
//!
//! These collections are **allocation-free**, `bytemuck`-compatible and suitable for deterministic, low-footprint
//! environments such as smart-contracts and embedded runtimes.
//!
//! ## Features
//!
//! ### Generic Collections
//!
//! - **[`FixedList<T, SIZE>`]** – contiguous array-backed list with `push`/`pop` semantics
//! - **[`FixedSet<T, SIZE>`]** – set-like structure for constant-size unique element storage
//! - **[`FixedBitSet<SIZE>`]** – bit-set-like structure for boolean flags at specific indices
//! - **[`PushPopCollection<T>`]** – trait abstraction over `push`, `pop`, `len` & slice access
//!
//! ### Utility Macros
//!
//! - **[`declare_fixed_array!`]** – generate a `Pod + Zeroable` struct wrapping a statically-sized array with a runtime length field
//! - **[`declare_fixed_option!`]** – generate a `Pod + Zeroable` Option-like wrapper with predictable layout
//! - **[`declare_fixed_set!`]** – generate a custom `Pod + Zeroable` fixed-capacity set type
//!
//! ## Safety & Compatibility
//!
//! - ✅ **100% safe Rust** with extensive unit tests
//! - ✅ **`bytemuck` compatible** for zero-copy serialization
//! - ✅ **Deterministic layout** suitable for on-chain storage
//! - ✅ **Allocation-free** operation
//!
//! ## Quick Examples
//!
//! ### Using FixedList
//!
//! ```rust
//! use saturn_collections::generic::fixed_list::FixedList;
//!
//! // A list that can hold up to 4 `u32`s without heap allocation
//! let mut list: FixedList<u32, 4> = FixedList::new();
//!
//! list.push(10).unwrap();
//! list.push(20).unwrap();
//! assert_eq!(list.len(), 2);
//! assert_eq!(list.pop(), Some(20));
//! ```
//!
//! ### Using FixedSet
//!
//! ```rust
//! use saturn_collections::generic::fixed_set::FixedSet;
//!
//! let mut set: FixedSet<u32, 8> = FixedSet::new();
//!
//! set.insert(3).unwrap();
//! set.insert(5).unwrap();
//! assert!(set.contains(&3));
//! assert_eq!(set.len(), 2);
//! ```
//!
//! ### Using FixedBitSet
//!
//! ```rust
//! use saturn_collections::generic::fixed_bitset::FixedBitSet;
//!
//! let mut bitset: FixedBitSet<16> = FixedBitSet::new();
//!
//! bitset.insert(3);
//! bitset.insert(7);
//! assert!(bitset.contains(3));
//! assert_eq!(bitset.count(), 2);
//! ```
//!
//! ### Using declare_fixed_array!
//!
//! ```rust
//! use saturn_collections::declare_fixed_array;
//!
//! // Create a wrapper that can store up to 16 `u64`s in a zero-copy buffer
//! declare_fixed_array!(U64Array16, u64, 16);
//!
//! let mut arr = U64Array16::new();
//! arr.add(42).unwrap();
//! assert_eq!(arr.len(), 1);
//! assert_eq!(arr.get(0), Some(&42));
//! ```
//!
//! ### Using declare_fixed_option!
//!
//! ```rust
//! use saturn_collections::declare_fixed_option;
//! use bytemuck::{Pod, Zeroable};
//!
//! #[repr(C)]
//! #[derive(Clone, Copy, Debug, Pod, Zeroable)]
//! struct Price(u64);
//!
//! // Wrapper with deterministic layout for on-chain storage
//! declare_fixed_option!(FixedPriceOpt, Price, 7);
//!
//! let some_price = FixedPriceOpt::some(Price(10_000));
//! assert!(some_price.is_some());
//! assert_eq!(some_price.get().unwrap().0, 10_000);
//! ```
//!
//! ### Using declare_fixed_set!
//!
//! ```rust,ignore
//! use saturn_collections::declare_fixed_set;
//! use bytemuck::{Pod, Zeroable};
//!
//! #[repr(C)]
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable, Default)]
//! struct AssetId(u64);
//!
//! // Create a custom set type with deterministic layout
//! declare_fixed_set!(AssetIdSet, AssetId, 10);
//!
//! let mut asset_set = AssetIdSet::new();
//! asset_set.insert(AssetId(1)).unwrap();
//! asset_set.insert(AssetId(2)).unwrap();
//! assert!(asset_set.contains(&AssetId(1)));
//! assert_eq!(asset_set.len(), 2);
//! ```
//!
//! ## Why Fixed-Size Collections?
//!
//! Smart-contract platforms (and many constrained systems) disallow dynamic memory allocation
//! or make it prohibitively expensive. Using compile-time capacities provides:
//!
//! - **Predictable layout** – crucial for zero-copy deserialization & account storage
//! - **Deterministic gas/fee usage** – no surprise allocation costs
//! - **Simpler safety audits** – bounded memory usage is easier to reason about
//! - **Performance** – no heap allocations or dynamic resizing overhead
//!
//! [`FixedList<T, SIZE>`]: generic::fixed_list::FixedList
//! [`FixedSet<T, SIZE>`]: generic::fixed_set::FixedSet
//! [`FixedBitSet<SIZE>`]: generic::fixed_bitset::FixedBitSet
//! [`PushPopCollection<T>`]: generic::push_pop::PushPopCollection
//! [`declare_fixed_array!`]: macro@declare_fixed_array
//! [`declare_fixed_option!`]: macro@declare_fixed_option
//! [`declare_fixed_set!`]: macro@declare_fixed_set

pub mod generic;
pub mod macros;
