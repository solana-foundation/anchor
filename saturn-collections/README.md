# Saturn Collections

Generic, fixed-capacity data structures & utility macros designed for **Arch / Saturn on-chain programs**.

These collections are **allocation-free**, `bytemuck`-compatible and suitable for deterministic, low-footprint environments such as smart-contracts and embedded runtimes.

---

## Features

### Generic Collections

- 🗃 **[FixedList]** – contiguous array-backed list with `push`/`pop` semantics
- 🗂 **[FixedSet]** – set-like structure for constant-size unique element storage
- 🔧 **[FixedBitSet]** – bit-set-like structure for boolean flags at specific indices
- ⚡ **[PushPopCollection]** – trait abstraction over `push`, `pop`, `len` & slice access

### Utility Macros

- ⚙️ **[declare_fixed_array!]** – generate a `Pod + Zeroable` struct wrapping a statically-sized array with a runtime length field
- ⚙️ **[declare_fixed_option!]** – generate a `Pod + Zeroable` Option-like wrapper with predictable layout
- ⚙️ **[declare_fixed_set!]** – generate a custom `Pod + Zeroable` fixed-capacity set type

### Safety & Compatibility

- ✅ **100% safe Rust** with extensive unit tests
- ✅ **`bytemuck` compatible** for zero-copy serialization
- ✅ **Deterministic layout** suitable for on-chain storage
- ✅ **Allocation-free** operation

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
saturn-collections = { git = "https://github.com/arch-protocol/saturn-arch-programs", package = "saturn-collections" }
```

The only direct dependencies are `bytemuck` and `serde` (both re-exported from the workspace).

## Quick Start

### FixedList

```rust
use saturn_collections::generic::fixed_list::FixedList;

// A list that can hold up to 4 `u32`s without heap allocation
let mut list: FixedList<u32, 4> = FixedList::new();

list.push(10).unwrap();
list.push(20).unwrap();
assert_eq!(list.len(), 2);
assert_eq!(list.pop(), Some(20));
assert_eq!(list.as_slice(), &[10]);
```

### FixedSet

```rust
use saturn_collections::generic::fixed_set::FixedSet;

let mut set: FixedSet<u32, 8> = FixedSet::new();

set.insert(3).unwrap();
set.insert(5).unwrap();
assert!(set.contains(&3));
assert_eq!(set.len(), 2);
assert_eq!(set.remove(&3), Some(3));
assert_eq!(set.len(), 1);
```

### FixedBitSet

```rust
use saturn_collections::generic::fixed_bitset::FixedBitSet;

let mut bitset: FixedBitSet<16> = FixedBitSet::new();

bitset.insert(3);
bitset.insert(7);
bitset.insert(15);
assert!(bitset.contains(3));
assert_eq!(bitset.count(), 3);

// Iterate over set bits
let bits: Vec<_> = bitset.iter().collect();
assert_eq!(bits, vec![3, 7, 15]);
```

### PushPopCollection Trait

```rust
use saturn_collections::generic::{fixed_list::FixedList, push_pop::PushPopCollection};

fn work_with_collection<T: PushPopCollection<i32>>(collection: &mut T) {
    collection.push(42).unwrap();
    collection.push(100).unwrap();
    assert_eq!(collection.len(), 2);
    assert_eq!(collection.pop(), Some(100));
}

let mut list = FixedList::<i32, 10>::new();
work_with_collection(&mut list);

let mut vec = Vec::new();
work_with_collection(&mut vec);
```

### declare_fixed_array!

```rust
use saturn_collections::declare_fixed_array;

// Create a wrapper that can store up to 16 `u64`s in a zero-copy buffer
declare_fixed_array!(U64Array16, u64, 16);

let mut arr = U64Array16::new();
arr.add(42).unwrap();
arr.add(100).unwrap();
assert_eq!(arr.len(), 2);
assert_eq!(arr.get(0), Some(&42));
assert_eq!(arr.get(1), Some(&100));

// Remove elements
assert_eq!(arr.remove_at(0), Some(42));
assert_eq!(arr.len(), 1);
assert_eq!(arr.get(0), Some(&100));
```

### declare_fixed_option!

```rust
use saturn_collections::declare_fixed_option;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Price(u64);

// Wrapper with deterministic layout for on-chain storage
declare_fixed_option!(FixedPriceOpt, Price, 7);

let some_price = FixedPriceOpt::some(Price(10_000));
assert!(some_price.is_some());
assert_eq!(some_price.get().unwrap().0, 10_000);

let none_price = FixedPriceOpt::none();
assert!(none_price.is_none());
assert_eq!(none_price.get(), None);

// Convert to/from Option
let std_option: Option<Price> = some_price.into();
assert_eq!(std_option, Some(Price(10_000)));
```

### declare_fixed_set!

```rust
use saturn_collections::declare_fixed_set;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable, Default)]
struct AssetId(u64);

// Create a custom set type with deterministic layout
declare_fixed_set!(AssetIdSet, AssetId, 10);

let mut asset_set = AssetIdSet::new();
asset_set.insert(AssetId(1)).unwrap();
asset_set.insert(AssetId(2)).unwrap();
assert!(asset_set.contains(&AssetId(1)));
assert_eq!(asset_set.len(), 2);

// Remove elements
assert_eq!(asset_set.remove(&AssetId(1)), Some(AssetId(1)));
assert_eq!(asset_set.len(), 1);
```

## Advanced Usage

### Insert or Modify Pattern

Both `FixedSet` and generated fixed sets support the `insert_or_modify` pattern:

```rust
use saturn_collections::generic::fixed_set::FixedSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct Counter {
    id: u32,
    count: u32,
}

let mut set: FixedSet<Counter, 10> = FixedSet::new();

// Insert or modify based on ID
set.insert_or_modify(
    Counter { id: 1, count: 1 },
    |existing| {
        existing.count += 1;
        Ok::<(), ()>(())
    }
).unwrap();

// Second call will modify the existing item
set.insert_or_modify(
    Counter { id: 1, count: 1 },
    |existing| {
        existing.count += 1;
        Ok::<(), ()>(())
    }
).unwrap();

assert_eq!(set.len(), 1);
assert_eq!(set.find(&Counter { id: 1, count: 0 }).unwrap().count, 2);
```

### Collecting Bits

```rust
use saturn_collections::generic::fixed_bitset::FixedBitSet;

let mut bitset: FixedBitSet<64> = FixedBitSet::new();
bitset.extend_from_slice(&[1, 5, 10, 63]);

let mut buffer = [0usize; 64];
let count = bitset.collect_sorted(&mut buffer);
assert_eq!(count, 4);
assert_eq!(&buffer[..count], &[1, 5, 10, 63]);
```

### Working with Slices

All collections provide efficient slice access:

```rust
use saturn_collections::generic::fixed_list::FixedList;

let mut list: FixedList<u32, 10> = FixedList::new();
list.push(1).unwrap();
list.push(2).unwrap();
list.push(3).unwrap();

// Get immutable slice
let slice = list.as_slice();
assert_eq!(slice, &[1, 2, 3]);

// Get mutable slice
let mut_slice = list.as_mut_slice();
mut_slice[0] = 100;
assert_eq!(list.as_slice(), &[100, 2, 3]);
```

## Why Fixed-Size Collections?

Smart-contract platforms (and many constrained systems) disallow dynamic memory allocation or make it prohibitively expensive. Using compile-time capacities provides:

- **Predictable layout** – crucial for zero-copy deserialization & account storage
- **Deterministic gas/fee usage** – no surprise allocation costs
- **Simpler safety audits** – bounded memory usage is easier to reason about
- **Performance** – no heap allocations or dynamic resizing overhead
- **Embedded compatibility** – suitable for `no_std` environments

## Memory Layout

All generated types maintain predictable memory layouts suitable for on-chain storage:

- **Fixed arrays** include padding for alignment
- **Fixed options** use a single byte flag plus padding
- **Fixed sets** pack elements contiguously with a length field
- All types are `Pod + Zeroable` for zero-copy serialization

## Error Handling

The library provides comprehensive error types:

- **`FixedListError`** – for list operations (currently only `Full`)
- **`FixedSetError`** – for set operations (`Full`, `Duplicate`)
- **`PushPopError`** – for generic collection operations (currently only `Full`)

All error types implement `std::fmt::Debug` and `std::fmt::Display` for easy debugging.

[FixedList]: https://docs.rs/saturn-collections/latest/saturn_collections/generic/fixed_list/struct.FixedList.html
[FixedSet]: https://docs.rs/saturn-collections/latest/saturn_collections/generic/fixed_set/struct.FixedSet.html
[FixedBitSet]: https://docs.rs/saturn-collections/latest/saturn_collections/generic/fixed_bitset/struct.FixedBitSet.html
[PushPopCollection]: https://docs.rs/saturn-collections/latest/saturn_collections/generic/push_pop/trait.PushPopCollection.html
[declare_fixed_array!]: https://docs.rs/saturn-collections/latest/saturn_collections/macro.declare_fixed_array.html
[declare_fixed_option!]: https://docs.rs/saturn-collections/latest/saturn_collections/macro.declare_fixed_option.html
[declare_fixed_set!]: https://docs.rs/saturn-collections/latest/saturn_collections/macro.declare_fixed_set.html
