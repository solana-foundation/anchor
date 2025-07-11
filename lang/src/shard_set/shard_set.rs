use core::marker::PhantomData;

use anchor_lang::prelude::AccountLoader;
use anchor_lang::prelude::Owner;
use anchor_lang::ZeroCopy;
use satellite_bitcoin::generic::fixed_list::FixedList;

use crate::error::Error;
use crate::error::ErrorCode;
use crate::shard_set::shard_handle::ShardHandle;
use crate::shard_set::shard_indices::IntoShardIndices;
use arch_program::program_error::ProgramError;
use core::cmp::Reverse;

/// Marker type representing an **unselected** set of shards.
pub struct Unselected;

/// Marker type representing a **selected** subset of shards.
pub struct Selected;

/// A type-safe wrapper around a slice of [`AccountLoader`]s representing the
/// shards that belong to the currently executing instruction.
///
/// # Typestate Pattern
///
/// Like its in-memory predecessor (`shard_set.rs`) this variant follows the
/// **typestate** pattern to ensure that callers always remember to narrow down
/// the set of shards they want to work with:
///
/// * [`Unselected`] – newly created `ShardSet` that exposes only a very small
///   API (`len`, `is_empty`, [`ShardSet::select_with`]) and therefore prevents accidental
///   mutations across *all* shards.
/// * [`Selected`] – returned by one of the selection helpers and unlocks the
///   full, high-level API (`selected_indices`, [`ShardSet::handle_by_index`],
///   [`ShardSet::for_each_mut`], …).
///
/// The compiler will make it impossible to call "selected-only" functions
/// unless `.select_with(…)` (or a future convenience wrapper) has been invoked
/// first.
///
/// # Generic Parameters
///
/// * `'slice` – lifetime of the input slice (`&'slice [AccountLoader<'info, S>]`).
/// * `'info` – lifetime that ties each `AccountLoader` to the surrounding
///   Anchor context.
/// * `S` – zero-copy shard type that must implement [`ZeroCopy`] + [`Owner`].
/// * `MAX_SELECTED_SHARDS` – upper bound enforced at **compile time** on how
///   many shards can participate in a single operation.
/// * `State` – either [`Unselected`] (default) or [`Selected`]; manipulated by
///   the public API and **never** supplied by callers.
///
/// # Example
///
/// ```rust,ignore
/// use saturn_account_shards::{ShardSet, Unselected};
/// # use anchor_lang::ZeroCopy;
/// # use anchor_lang::prelude::Owner;
///
/// # #[derive(Default, Clone, Copy)]
/// # #[repr(C)]
/// # struct DummyShard; // implements ZeroCopy + Owner
/// # unsafe impl ZeroCopy for DummyShard {}
/// # impl Owner for DummyShard { const OWNER: anchor_lang::solana_program::pubkey::Pubkey = anchor_lang::solana_program::pubkey::Pubkey::new_from_array([0u8; 32]); }
///
/// // Imagine these loaders coming from the instruction's account context.
/// fn example<'info>(loaders: &'info [&'info AccountLoader<'info, DummyShard>]) {
///     // Create an *unselected* ShardSet
///     let shards = ShardSet::<DummyShard, 4>::from_loaders(loaders);
///
///     // Pick the shards we actually want to touch – everything else stays immutable
///     let selected = shards.select_with([0, 2]).expect("invalid selection");
///
///     // Do something with the selected shards
///     selected
///         .for_each_mut(|shard| {
///             // mutate shard...
///         })
///         .unwrap();
/// }
/// ```
///
/// Internally **no long-lived `Ref` or `RefMut` is held**. Every call borrows a
/// shard only for the exact duration of the closure passed to
/// [`ShardHandle::with_ref`] / [`ShardHandle::with_mut`], making it impossible
/// to accidentally lock up accounts for longer than necessary.
pub struct ShardSet<'slice, 'info, S, const MAX_SELECTED_SHARDS: usize, State = Unselected>
where
    S: ZeroCopy + Owner,
{
    /// All shard loaders supplied by the caller.
    loaders: &'slice [AccountLoader<'info, S>],

    /// Indexes of the shards that are currently *selected* (may be empty while
    /// the set is in the [`Unselected`] state).
    selected: FixedList<usize, MAX_SELECTED_SHARDS>,

    /// Typestate marker.
    _state: PhantomData<State>,
}

// ---------------------------- Unselected ------------------------------------
impl<'slice, 'info, S, const MAX_SELECTED_SHARDS: usize>
    ShardSet<'slice, 'info, S, MAX_SELECTED_SHARDS, Unselected>
where
    S: ZeroCopy + Owner,
{
    /// Creates a new `ShardSet` wrapping the provided loaders.
    #[inline]
    pub fn from_loaders(loaders: &'slice [AccountLoader<'info, S>]) -> Self {
        Self {
            loaders,
            selected: FixedList::new(),
            _state: PhantomData,
        }
    }

    /// Number of shards (loaders) available.
    #[inline]
    pub fn len(&self) -> usize {
        self.loaders.len()
    }

    /// `true` if no shards are present.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.loaders.is_empty()
    }
}

// ----------------- Unselected -> Selected -------------------------
impl<'slice, 'info, S, const MAX_SELECTED_SHARDS: usize>
    ShardSet<'slice, 'info, S, MAX_SELECTED_SHARDS, Unselected>
where
    S: ZeroCopy + Owner,
{
    #[inline]
    pub fn from(slice: &'slice [AccountLoader<'info, S>]) -> Self {
        ShardSet::from_loaders(slice)
    }

    /// Select shards by index and transition into the [`Selected`] state.
    ///
    /// Only available on *writable* shard sets.
    ///
    /// # Errors
    /// * [`StateShardError::TooManyShardsSelected`]
    ///   if the number of indices in `spec` exceeds `MAX_SELECTED_SHARDS`.
    /// * [`StateShardError::DuplicateShardSelection`]
    ///   if `spec` contains the same index more than once.
    pub fn select_with<T>(
        mut self,
        spec: T,
    ) -> crate::Result<ShardSet<'slice, 'info, S, MAX_SELECTED_SHARDS, Selected>>
    where
        T: IntoShardIndices<MAX_SELECTED_SHARDS>,
    {
        let indexes = spec
            .into_indices()
            .map_err(|_| Error::from(ErrorCode::TooManyShardsSelected))?;

        for &idx in indexes.as_slice() {
            if idx >= self.loaders.len() {
                return Err(ErrorCode::OutOfBounds.into());
            }

            // Avoid selecting the same shard more than once – this could lead to
            // double-counting in accounting helpers further down the stack.
            if self.selected.iter().any(|&existing| existing == idx) {
                return Err(ErrorCode::DuplicateShardSelection.into());
            }

            self.selected
                .push(idx)
                .map_err(|_| Error::from(ErrorCode::TooManyShardsSelected))?;
        }

        Ok(ShardSet {
            loaders: self.loaders,
            selected: self.selected,
            _state: PhantomData,
        })
    }

    /// Select the shard that minimises the value returned by `key_fn`.
    ///
    /// # Errors
    /// Propagates the same conditions as [`Self::select_with`].
    pub fn select_min_by<F>(
        self,
        key_fn: F,
    ) -> crate::Result<ShardSet<'slice, 'info, S, MAX_SELECTED_SHARDS, Selected>>
    where
        F: Fn(&S) -> u64,
    {
        // Identify the shard that yields the smallest key according to `key_fn`.
        let mut best_idx: Option<usize> = None;
        let mut best_key: u64 = u64::MAX;

        for (idx, loader) in self.loaders.iter().enumerate() {
            let handle = ShardHandle::new(loader);
            // Ignore shards that cannot be loaded (e.g. runtime borrow failure).
            if let Ok(key) = handle.with_ref(|shard| key_fn(shard)) {
                if key < best_key {
                    best_key = key;
                    best_idx = Some(idx);
                }
            }
        }

        match best_idx {
            Some(i) => self.select_with([i]),
            None => self.select_with([]),
        }
    }

    // ---------------------------------------------------------------------
    /// Select all shards that satisfy the provided `predicate`.
    ///
    /// # Errors
    /// Propagates the same conditions as [`Self::select_with`].
    pub fn select_multiple_by<P>(
        self,
        predicate: P,
    ) -> crate::Result<ShardSet<'slice, 'info, S, MAX_SELECTED_SHARDS, Selected>>
    where
        P: Fn(&S) -> bool,
    {
        let mut indices = Vec::new();
        for (idx, loader) in self.loaders.iter().enumerate() {
            let handle = ShardHandle::new(loader);
            if let Ok(pass) = handle.with_ref(|shard| predicate(shard)) {
                if pass {
                    indices.push(idx);
                }
            }
        }
        self.select_with(indices)
    }

    // ---------------------------------------------------------------------
    /// Select a minimal set of shards – ordered by `key_fn` (descending) –
    /// such that the accumulated `predicate` evaluates to `true`.
    ///
    /// The `predicate` receives the **current total key value** accumulated
    /// across the candidate shards.  This keeps the implementation borrow-
    /// checker friendly while still covering many practical use-cases
    /// (e.g. _select shards until their total BTC amount is above some
    /// threshold_).
    ///
    /// # Errors
    /// Propagates the same conditions as [`Self::select_with`].
    pub fn select_multiple_sorted<K, P>(
        self,
        key_fn: K,
        predicate: P,
    ) -> crate::Result<ShardSet<'slice, 'info, S, MAX_SELECTED_SHARDS, Selected>>
    where
        K: Fn(&S) -> u64,
        P: Fn(u64) -> bool,
    {
        // 1. Gather all shard indices and sort them **descending** by `key_fn`.
        let mut indices: Vec<usize> = (0..self.len()).collect();
        indices.sort_by_key(|&i| {
            let handle = ShardHandle::new(&self.loaders[i]);
            let key = handle.with_ref(|shard| key_fn(shard)).unwrap_or(0);
            // Reverse to get descending order.
            Reverse(key)
        });

        // 2. Incrementally add shards until the predicate is satisfied.
        let mut selected = Vec::new();
        let mut accumulated: u64 = 0;
        for &idx in &indices {
            let handle = ShardHandle::new(&self.loaders[idx]);
            accumulated += handle.with_ref(|shard| key_fn(shard)).unwrap_or(0);
            selected.push(idx);
            if predicate(accumulated) {
                return self.select_with(selected);
            }
        }

        // Fallback – select everything (may still error if `MAX_SELECTED_SHARDS` exceeded).
        self.select_with(indices)
    }
}

// ---------------------------- Selected -------------------------------
impl<'slice, 'info, S, const MAX_SELECTED_SHARDS: usize>
    ShardSet<'slice, 'info, S, MAX_SELECTED_SHARDS, Selected>
where
    S: ZeroCopy + Owner,
{
    /// Returns the indexes that were selected via [`Self::select_with`].
    #[inline]
    pub fn selected_indices(&self) -> &[usize] {
        self.selected.as_slice()
    }

    /// Returns a [`ShardHandle`] for the shard at **global** `idx`.
    #[inline]
    pub fn handle_by_index(&self, idx: usize) -> ShardHandle<'slice, 'info, S> {
        debug_assert!(idx < self.loaders.len());
        ShardHandle::new(&self.loaders[idx])
    }

    /// Executes `f` for every **selected** shard, borrowing each one exactly
    /// for the duration of the closure call. Only available on *writable*
    /// shard sets.
    pub fn for_each<R>(&self, mut f: impl FnMut(&S) -> R) -> Result<Vec<R>, ProgramError> {
        let mut results = Vec::with_capacity(self.selected.len());
        for &idx in self.selected.iter() {
            let handle = ShardHandle::new(&self.loaders[idx]);
            let out = handle.with_ref(|shard| f(shard))?;
            results.push(out);
        }
        Ok(results)
    }
}

// ------------------------ Selected (mutable helper) --------------------------------
impl<'slice, 'info, S, const MAX_SELECTED_SHARDS: usize>
    ShardSet<'slice, 'info, S, MAX_SELECTED_SHARDS, Selected>
where
    S: ZeroCopy + Owner,
{
    /// Executes `f` for every **selected** shard mutably.
    pub fn for_each_mut<R>(&self, mut f: impl FnMut(&mut S) -> R) -> Result<Vec<R>, ProgramError> {
        let mut results = Vec::with_capacity(self.selected.len());
        for &idx in self.selected.iter() {
            let handle = ShardHandle::new(&self.loaders[idx]);
            let out = handle.with_mut(|shard| f(shard))?;
            results.push(out);
        }
        Ok(results)
    }
}

impl<'slice, 'info, S, const MAX_SELECTED_SHARDS: usize>
    ShardSet<'slice, 'info, S, MAX_SELECTED_SHARDS, Selected>
where
    S: ZeroCopy + Owner,
{
    /// Redistribute the remaining satoshi value that belongs to the selected shards
    /// back into new outputs, updating the provided `TransactionBuilder` in the
    /// process.
    ///
    /// This is a thin wrapper around
    /// `crate::split::redistribute_remaining_btc_to_shards`. See that function
    /// for detailed documentation of the parameters and error semantics.
    pub fn redistribute_remaining_btc_to_shards<
        const MAX_USER_UTXOS: usize,
        const MAX_SHARDS_PER_PROGRAM: usize,
        RS,
        U,
    >(
        &mut self,
        tx_builder: &mut satellite_bitcoin::TransactionBuilder<
            MAX_USER_UTXOS,
            MAX_SHARDS_PER_PROGRAM,
            RS,
        >,
        removed_from_shards: u64,
        program_script_pubkey: bitcoin::ScriptBuf,
        fee_rate: &satellite_bitcoin::fee_rate::FeeRate,
    ) -> Result<Vec<u128>, satellite_bitcoin::MathError>
    where
        RS: satellite_bitcoin::generic::fixed_set::FixedCapacitySet<
                Item = arch_program::rune::RuneAmount,
            > + Default,
        U: satellite_bitcoin::utxo_info::UtxoInfoTrait<RS>,
        S: super::StateShard<U, RS> + ZeroCopy + Owner,
    {
        super::split::redistribute_remaining_btc_to_shards::<
            MAX_USER_UTXOS,
            MAX_SHARDS_PER_PROGRAM,
            RS,
            U,
            S,
            MAX_SELECTED_SHARDS,
        >(
            tx_builder,
            self,
            removed_from_shards,
            program_script_pubkey,
            fee_rate,
        )
    }

    /// Compute the number of satoshis that are still unsettled in the selected
    /// shards, i.e. need to be returned after accounting for fees and any
    /// amounts that were already removed by the caller.
    pub fn compute_unsettled_btc_in_shards<
        const MAX_USER_UTXOS: usize,
        const MAX_SHARDS_PER_PROGRAM: usize,
        RS,
        U,
    >(
        &self,
        tx_builder: &satellite_bitcoin::TransactionBuilder<
            MAX_USER_UTXOS,
            MAX_SHARDS_PER_PROGRAM,
            RS,
        >,
        removed_from_shards: u64,
        fee_rate: &satellite_bitcoin::fee_rate::FeeRate,
    ) -> Result<u64, satellite_bitcoin::MathError>
    where
        RS: satellite_bitcoin::generic::fixed_set::FixedCapacitySet<
                Item = arch_program::rune::RuneAmount,
            > + Default,
        U: satellite_bitcoin::utxo_info::UtxoInfoTrait<RS>,
        S: super::StateShard<U, RS> + ZeroCopy + Owner,
    {
        super::split::compute_unsettled_btc_in_shards::<
            MAX_USER_UTXOS,
            MAX_SHARDS_PER_PROGRAM,
            RS,
            U,
            S,
            MAX_SELECTED_SHARDS,
        >(tx_builder, self, removed_from_shards, fee_rate)
    }

    /// Update all shards after a transaction has been constructed, signed and
    /// broadcast. Internally forwards to
    /// `crate::update::update_shards_after_transaction`.
    pub fn update_shards_after_transaction<
        const MAX_USER_UTXOS: usize,
        const MAX_SHARDS_PER_PROGRAM: usize,
        RS,
        U,
    >(
        &self,
        tx_builder: &mut satellite_bitcoin::TransactionBuilder<
            MAX_USER_UTXOS,
            MAX_SHARDS_PER_PROGRAM,
            RS,
        >,
        program_script_pubkey: &bitcoin::ScriptBuf,
        fee_rate: &satellite_bitcoin::fee_rate::FeeRate,
    ) -> crate::Result<()>
    where
        RS: satellite_bitcoin::generic::fixed_set::FixedCapacitySet<
                Item = arch_program::rune::RuneAmount,
            > + Default,
        U: satellite_bitcoin::utxo_info::UtxoInfoTrait<RS>,
        S: super::StateShard<U, RS> + ZeroCopy + Owner,
    {
        super::update::update_shards_after_transaction::<
            MAX_USER_UTXOS,
            MAX_SHARDS_PER_PROGRAM,
            MAX_SELECTED_SHARDS,
            RS,
            U,
            S,
        >(tx_builder, self, program_script_pubkey, fee_rate)
    }

    // === Rune-specific helpers (compiled only with `runes` feature) =============
    #[cfg_attr(docsrs, doc(cfg(feature = "runes")))]
    #[cfg(feature = "runes")]
    /// Compute the yet-to-be-settled Rune amounts held by the selected shards.
    pub fn compute_unsettled_rune_in_shards<RS, U>(
        &self,
        removed_from_shards: RS,
    ) -> Result<RS, super::StateShardError>
    where
        RS: satellite_bitcoin::generic::fixed_set::FixedCapacitySet<
                Item = arch_program::rune::RuneAmount,
            > + Default,
        U: satellite_bitcoin::utxo_info::UtxoInfoTrait<RS>,
        S: super::StateShard<U, RS> + ZeroCopy + Owner,
    {
        super::split::compute_unsettled_rune_in_shards::<RS, U, S, MAX_SELECTED_SHARDS>(
            self,
            removed_from_shards,
        )
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "runes")))]
    #[cfg(feature = "runes")]
    /// Redistribute the remaining Rune amounts back to the shards, generating
    /// the appropriate runestone edicts in the provided `TransactionBuilder`.
    pub fn redistribute_remaining_rune_to_shards<
        const MAX_USER_UTXOS: usize,
        const MAX_SHARDS_PER_PROGRAM: usize,
        RS,
        U,
    >(
        &mut self,
        tx_builder: &mut satellite_bitcoin::TransactionBuilder<
            MAX_USER_UTXOS,
            MAX_SHARDS_PER_PROGRAM,
            RS,
        >,
        removed_from_shards: RS,
        program_script_pubkey: bitcoin::ScriptBuf,
    ) -> Result<Vec<RS>, super::StateShardError>
    where
        RS: satellite_bitcoin::generic::fixed_set::FixedCapacitySet<
                Item = arch_program::rune::RuneAmount,
            > + Default,
        U: satellite_bitcoin::utxo_info::UtxoInfoTrait<RS>,
        S: super::StateShard<U, RS> + ZeroCopy + Owner,
    {
        super::split::redistribute_remaining_rune_to_shards::<
            MAX_USER_UTXOS,
            MAX_SHARDS_PER_PROGRAM,
            RS,
            U,
            S,
            MAX_SELECTED_SHARDS,
        >(tx_builder, self, removed_from_shards, program_script_pubkey)
    }
}
