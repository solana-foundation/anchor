# Saturn Safe Math

A tiny, dependency-light Rust crate that provides **overflow-safe arithmetic helpers** for fixed-width integer types. It is part of the **Saturn** suite of on-chain programs and tooling, but can be used in any Rust project that needs extra safety when manipulating numerical values.

---

## Why?

Rust's standard library already panics on debug builds when an arithmetic operation overflows, but it **wraps silently in release mode**. In financial or blockchain contexts this is unacceptable — a single unchecked overflow can lead to loss of funds or inconsistent state.

`saturn-safe-math` wraps the checked arithmetic traits from the [`num`](https://crates.io/crates/num) crate in a tiny, ergonomic API that:

* Returns a `Result<T, MathError>` instead of panicking or wrapping.
* Works with any integer type that implements the `Checked*` traits (`u8`, `u16`, `u32`, `u64`, `u128`, custom new-types, …).
* Offers a `mul_div` helper that performs multiplication in 256-bit space before dividing, eliminating intermediate overflow.
* Converts errors into `u32` codes so they can be bubbled up easily from smart-contract code that expects numeric error values.

---

## Installation

Add the crate to your `Cargo.toml` (the crate is published **within the workspace**, so you usually depend on it with a workspace path):

```toml
[dependencies]
saturn-safe-math = { path = "crates/saturn-safe-math" }
```

If pulling from `crates.io`:

```toml
[dependencies]
saturn-safe-math = "0.1"
```

The crate is `#![no_std]` compatible (it relies on `thiserror` which also supports `no_std`).

---

## Quick start

```rust
use saturn_safe_math::{safe_add, safe_sub, safe_mul, safe_div, mul_div, MathError};

fn main() -> Result<(), MathError> {
    let a: u64 = 10;
    let b: u64 = 20;

    // Basic operations
    let sum        = safe_add(a, b)?;         // 30
    let diff       = safe_sub(sum, 5u64)?;    // 25
    let product    = safe_mul(diff, 3u64)?;   // 75
    let quotient   = safe_div(product, 5u64)?; // 15

    // Full-precision multiply-then-divide without overflow
    let precise = mul_div(1_000_000_u128, 3333_u128, 1_000_u128)?; // 3_333_000

    println!("sum={sum}, diff={diff}, product={product}, quotient={quotient}, precise={precise}");
    Ok(())
}
```

Any attempt that would overflow (or divide by zero) returns a `MathError` instead of wrapping.

---

## API at a glance

```
Result<T, MathError> safe_add<T: CheckedAdd>(a, b)
Result<T, MathError> safe_sub<T: CheckedSub>(a, b)
Result<T, MathError> safe_mul<T: CheckedMul>(a, b)
Result<T, MathError> safe_div<T: CheckedDiv>(a, b)

// multiply a * b exactly using U256, then divide the result by `div`
Result<T, MathError> mul_div<T: TryFrom<U256>>(mul_a, mul_b, div)
```

### `MathError`

```rust
#[derive(Error, Debug, PartialEq)]
pub enum MathError {
    AdditionOverflow,
    SubtractionOverflow,
    MultiplicationOverflow,
    DivisionOverflow,
    ConversionError,
}
```

Each variant implements `Display` (via `thiserror`) and converts into a `u32` so it can be returned from smart contracts:

```rust
let err_code: u32 = MathError::AdditionOverflow.into();
assert_eq!(err_code, 6000);
```

| Error | Code |
|-------|------|
| AdditionOverflow | 6000 |
| SubtractionOverflow | 6001 |
| MultiplicationOverflow | 6002 |
| DivisionOverflow | 6003 |
| ConversionError | 6004 |

---

## `no_std`

The crate is `#![no_std]`-ready — simply disable default features of its dependencies where needed and compile for your target.
