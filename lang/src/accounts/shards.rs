//! Variable-length collection of account shards
//!
//! Anchor traditionally requires every account that an instruction touches to be listed as a
//! field in the `#[derive(Accounts)]` struct.  While this works well for a **fixed** and usually
//! small number of accounts, it quickly becomes impractical for programs that need to
//! atomically update *dozens* of similar accounts – for example when distributing rewards across
//! a dynamic list of liquidity-provider positions.
//!
//! Another concrete use-case are programs that **bridge to Bitcoin**.  Each on-chain UTXO is
//! modeled as its own Solana account (often an `AccountLoader<BtcUtxo>`), because the serialized
//! state of a single UTXO must remain small and independently updatable.  When building a
//! BTC-spending transaction the program has to choose *some* of those UTXO accounts – but not
//! necessarily all – based on external mempool rules such as the
//! [25 ascendants / 25 descendants](https://bitcoinops.org/en/topics/transaction-chain-limits/) limit or
//! fee-bump constraints.  Passing the UTXOs as `Shards` gives the instruction a flexible yet
//! type-safe way to work with an arbitrary subset:
//!
//! ```ignore
//! #[derive(Accounts)]
//! pub struct SpendUtxos<'info> {
//!     /// Up to ten UTXO shards that will be aggregated into a single BTC transaction.
//!     #[account(shards = 10, seeds = [b"utxo"], bump)]
//!     pub utxos: Shards<'info, AccountLoader<'info, BtcUtxo>>,
//! }
//! ```
//!
//! Inside the instruction handler you can iterate over `utxos` just like over a `Vec<_>` and
//! decide which shards to consume depending on real-time fee rates, child-pays-for-parent (CPFP)
//! strategies, or any other mempool heuristic.
//!
//! The `Shards<'info, T>` container solves this scalability problem.  It is a thin, ergonomic
//! wrapper around `Vec<T>` that implements all Anchor traits necessary to act as **one** field in
//! an accounts struct, while logically representing **many** homogeneous accounts underneath.
//! Every element of the inner vector – each individual *shard* – can itself be an
//! `Account<'info, U>`, an `AccountLoader<'info, U>`, or any other container that implements
//! `Accounts`.
//!
//! ```ignore
//! use satellite_lang::prelude::*;
//! use satellite_lang::accounts::shards::Shards;
//!
//! #[account]
//! pub struct Position { /* … */ }
//!
//! #[derive(Accounts)]
//! pub struct DistributeRewards<'info> {
//!     /// Signer paying the fees
//!     pub payer: Signer<'info>,
//!
//!     /// *All* position shards belonging to the pool.  We do **not** know the exact number at
//!     /// compile time – it depends on how many users have provided liquidity – therefore we use
//!     /// `shards = "rest"` so Anchor keeps consuming accounts until the slice is empty.
//!     #[account(seeds = [b"position"], bump, shards = "rest")]
//!     pub positions: Shards<'info, Account<'info, Position>>,
//! }
//! ```
//!
//! ### Using the `shards` constraint
//!
//! A field whose type is `Shards<'info, T>` **must** be annotated with `#[account(shards = ...)]`.
//! There are two supported forms:
//!
//! * `shards = N` – where `N` is a *compile-time* upper bound (`u8` literal ≤ 50) on how many
//!   shards this instruction will accept.  Anchor will verify that exactly `N` shards are present
//!   and emit a descriptive error otherwise.
//! * `shards = "rest"` – indicates that the field should keep consuming accounts until no more
//!   input accounts are left.  Because it acts as a catch-all, it **must** be the *last* field in
//!   the accounts struct.
//!
//! ### PDA seed integration
//!
//! When a shard participates in PDA derivation via `seeds = [...]`, Anchor automatically pushes
//! the **current shard index** (`u64`, little-endian) onto the seed stack *before* verifying the
//! seeds of that shard and pops it afterwards.  This enables deterministic PDA derivation such as
//! `seed = [b"shard", shard_index]` without any extra boilerplate.
//!
//! ### Trait implementations
//!
//! Besides `Accounts`, `Shards` implements `Deref`, `DerefMut`, all common iterator traits and
//! forwards `ToAccountInfos` / `ToAccountMetas` to every inner element, so you can treat a value
//! of type `Shards<'info, T>` just like a regular `Vec<T>` in almost all situations.
//!
//! ### When *not* to use Shards
//!
//! If the number of accounts is known at compile time or very small (≤ 4), prefer listing the
//! fields individually – it results in simpler code and less validation overhead.
//!
//! ---
//!
//! For a full-featured example, see the *btc_tx* integration tests where builder accounts are
//! grouped via `Shards` and manipulated through the helper types in the
//! `saturn-account-shards` crate.
//!
//! ---
//!
//! *Module generated automatically – please keep in sync with `#[account(shards = ...)]`
//! validation logic in `satellite-lang-syn`.*
pub struct Shards<'info, T> {
    /// Vector of shards – each shard is itself an account container (Account, AccountLoader, …).
    pub shards: Vec<T>,
    // Tie the lifetime parameter `'info` to the struct without storing an actual reference.
    _marker: core::marker::PhantomData<&'info ()>,
}

impl<'info, T> Shards<'info, T> {
    /// Creates a new `Shards` value from the given vector.
    pub fn new(shards: Vec<T>) -> Self {
        Self {
            shards,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'info, T> core::ops::Deref for Shards<'info, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.shards
    }
}

impl<'info, T> core::ops::DerefMut for Shards<'info, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.shards
    }
}

impl<'info, T> IntoIterator for Shards<'info, T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.shards.into_iter()
    }
}

impl<'info, T: crate::ToAccountInfos<'info>> crate::ToAccountInfos<'info> for Shards<'info, T> {
    fn to_account_infos(&self) -> Vec<arch_program::account::AccountInfo<'info>> {
        self.shards
            .iter()
            .flat_map(|s| s.to_account_infos())
            .collect()
    }
}

impl<'info, T: crate::ToAccountMetas> crate::ToAccountMetas for Shards<'info, T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<arch_program::account::AccountMeta> {
        self.shards
            .iter()
            .flat_map(|s| s.to_account_metas(is_signer))
            .collect()
    }
}

impl<'info, T> core::ops::Index<usize> for Shards<'info, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.shards[index]
    }
}

impl<'info, T> core::ops::IndexMut<usize> for Shards<'info, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.shards[index]
    }
}

pub trait ShardIndexBumps {
    /// Pushes the given shard index (as a little-endian `u64`) onto the seed stack.
    fn push_shard_index(&mut self, idx: u64);
    /// Pops the most recently pushed shard index from the stack.
    fn pop_shard_index(&mut self);
}

// Blanket Accounts implementation that simply consumes *all* remaining accounts and
// groups them into shards, relying on the inner `T` to decide how many accounts it
// needs.  The macro-generated code for the account struct will typically build the
// `Shards` value directly, but having this impl makes the type usable on its own.
impl<'info, B, T> crate::Accounts<'info, B> for Shards<'info, T>
where
    T: crate::Accounts<'info, B>,
    B: ShardIndexBumps,
{
    fn try_accounts(
        program_id: &arch_program::pubkey::Pubkey,
        accounts: &mut &'info [arch_program::account::AccountInfo<'info>],
        ix_data: &[u8],
        bumps: &mut B,
        reallocs: &mut std::collections::BTreeSet<arch_program::pubkey::Pubkey>,
    ) -> crate::Result<Self> {
        let mut vec = Vec::new();
        let mut shard_idx: u64 = 0;
        while !accounts.is_empty() {
            let before_len = accounts.len();

            // Push the current index so inner `try_accounts` can include it in PDA seeds.
            bumps.push_shard_index(shard_idx);
            let item = T::try_accounts(program_id, accounts, ix_data, bumps, reallocs)?;
            bumps.pop_shard_index();

            // Ensure progress – `T::try_accounts` must shrink the slice.
            if accounts.len() == before_len {
                return Err(crate::error::ErrorCode::ShardsInnerDidNotConsume.into());
            }

            vec.push(item);
            shard_idx += 1;
        }
        Ok(Self {
            shards: vec,
            _marker: core::marker::PhantomData,
        })
    }
}

impl<'info, T: core::fmt::Debug> core::fmt::Debug for Shards<'info, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Shards")
            .field("shards", &self.shards)
            .finish()
    }
}

impl<'info, T> core::default::Default for Shards<'info, T> {
    fn default() -> Self {
        Self {
            shards: Vec::new(),
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'info, T> core::convert::AsRef<[T]> for Shards<'info, T> {
    fn as_ref(&self) -> &[T] {
        &self.shards
    }
}

impl<'info, T> core::convert::AsMut<[T]> for Shards<'info, T> {
    fn as_mut(&mut self) -> &mut [T] {
        &mut self.shards
    }
}

impl<'info, T> core::convert::From<Vec<T>> for Shards<'info, T> {
    fn from(shards: Vec<T>) -> Self {
        Self::new(shards)
    }
}

impl<'info, T> core::iter::FromIterator<T> for Shards<'info, T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

impl<'info, T> core::iter::Extend<T> for Shards<'info, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.shards.extend(iter);
    }
}

impl<'a, 'info, T> core::iter::IntoIterator for &'a Shards<'info, T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.shards.iter()
    }
}

impl<'a, 'info, T> core::iter::IntoIterator for &'a mut Shards<'info, T> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.shards.iter_mut()
    }
}
