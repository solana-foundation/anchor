---
title: Space Reference
description: Anchor - Space Reference
---

This reference tells you how much space you should allocate for an account.

---

This only applies to accounts that don't use `zero-copy`. `zero-copy` uses `repr(C)` with a pointer cast,
so there the `C` layout applies.

In addition to the space for the account data, you have to add `8` to the `space` constraint for Anchor's internal discriminator (see the example).

| Types      | Space in bytes                | Details/Example                                                                                 |
| ---------- | ----------------------------- | ----------------------------------------------------------------------------------------------- |
| bool       | 1                             | would only require 1 bit but still uses 1 byte                                                  |
| u8/i8      | 1                             |
| u16/i16    | 2                             |
| u32/i32    | 4                             |
| u64/i64    | 8                             |
| u128/i128  | 16                            |
| [T;amount] | space(T) \* amount            | e.g. space([u16;32]) = 2 \* 32 = 64                                                             |
| Pubkey     | 32                            |
| Vec\<T>    | 4 + (space(T) \* amount)      | Account size is fixed so account should be initialized with sufficient space from the beginning |
| String     | 4 + length of string in bytes | Account size is fixed so account should be initialized with sufficient space from the beginning |
| Option\<T> | 1 + (space(T))                |
| Enum       | 1 + Largest Variant Size      | e.g. Enum { A, B { val: u8 }, C { val: u16 } } -> 1 + space(u16) = 3                            |
| f32        | 4                             | serialization will fail for NaN                                                                 |
| f64        | 8                             | serialization will fail for NaN                                                                 |

# Example

```rust
#[account]
pub struct MyData {
    pub val: u16,
    pub state: GameState,
    pub players: Vec<Pubkey> // we want to support up to 10 players
}

impl MyData {
    pub const MAX_SIZE: usize = 2 + (1 + 32) + (4 + 10 * 32);
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum GameState {
    Active,
    Tie,
    Won { winner: Pubkey },
}

#[derive(Accounts)]
pub struct InitializeMyData<'info> {
    // Note that we have to add 8 to the space for the internal anchor
    #[account(init, payer = signer, space = 8 + MyData::MAX_SIZE)]
    pub acc: Account<'info, MyData>,
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}
```
