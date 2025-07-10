use arch_program::rune::{RuneAmount, RuneId};

use arch_program::{
    program::get_bitcoin_tx_output_value, program_error::ProgramError, utxo::UtxoMeta,
};

use bytemuck::{Pod, Zeroable};
use saturn_collections::declare_fixed_array;
use saturn_collections::declare_fixed_option;
use saturn_collections::generic::fixed_set::{FixedCapacitySet, FixedSet};

use crate::{bytes::txid_to_bytes_big_endian, error::BitcoinTxError};

#[cfg(feature = "runes")]
use crate::arch::get_runes;

/// Trait defining the essential operations needed by StateShard for UTXO types.
/// This allows StateShard to work with different concrete UtxoInfo implementations
/// while maintaining a consistent interface.
pub trait UtxoInfoTrait<RuneSet: FixedCapacitySet<Item = RuneAmount>> {
    /// Create a new UtxoInfo with the given metadata and value.
    fn new(meta: UtxoMeta, value: u64) -> Self;

    /// Get the UTXO metadata (txid, vout, etc.)
    fn meta(&self) -> &UtxoMeta;

    /// Get the satoshi value of this UTXO
    fn value(&self) -> u64;

    /// Check if this UTXO is equal to another based on metadata
    fn eq_meta(&self, other: &Self) -> bool;

    /// Get access to the runes information when the "runes" feature is enabled
    #[cfg(feature = "runes")]
    fn runes(&self) -> &RuneSet;

    /// Get mutable access to the runes information when the "runes" feature is enabled
    #[cfg(feature = "runes")]
    fn runes_mut(&mut self) -> &mut RuneSet;

    /// Get access to the consolidation information when the "utxo-consolidation" feature is enabled
    #[cfg(feature = "utxo-consolidation")]
    fn needs_consolidation(&self) -> &FixedOptionF64;

    /// Get mutable access to the consolidation information when the "utxo-consolidation" feature is enabled
    #[cfg(feature = "utxo-consolidation")]
    fn needs_consolidation_mut(&mut self) -> &mut FixedOptionF64;
}

#[cfg(feature = "utxo-consolidation")]
declare_fixed_option!(FixedOptionF64, f64, 7);

#[cfg(feature = "runes")]
pub type SingleRuneSet = FixedSet<RuneAmount, 1>;

#[cfg(not(feature = "runes"))]
pub type SingleRuneSet = FixedSet<RuneAmount, 0>;

#[repr(C, align(8))]
#[derive(Clone, Copy, Debug)]
// Provide a default generic parameter so callers can simply use `UtxoInfo` without specifying
// the rune set type. When the `runes` feature is enabled the default is `SingleRuneSet`; when it
// is disabled the default is `()`.
pub struct UtxoInfo<RuneSet: FixedCapacitySet<Item = RuneAmount> = SingleRuneSet> {
    pub meta: UtxoMeta,
    pub value: u64,

    #[cfg(feature = "runes")]
    pub runes: RuneSet,

    #[cfg(feature = "utxo-consolidation")]
    pub needs_consolidation: FixedOptionF64,

    // Ensure the generic parameter is referenced even when the `runes` feature is disabled.
    #[cfg(not(feature = "runes"))]
    _phantom: std::marker::PhantomData<RuneSet>,
}

// Implement the UtxoInfoTrait for UtxoInfo
impl<RuneSet: FixedCapacitySet<Item = RuneAmount>> UtxoInfoTrait<RuneSet> for UtxoInfo<RuneSet> {
    fn new(meta: UtxoMeta, value: u64) -> Self {
        Self {
            meta,
            value,
            ..Default::default()
        }
    }

    fn meta(&self) -> &UtxoMeta {
        &self.meta
    }

    fn value(&self) -> u64 {
        self.value
    }

    fn eq_meta(&self, other: &Self) -> bool {
        self.meta == other.meta
    }

    /// Get access to the runes information when the "runes" feature is enabled
    #[cfg(feature = "runes")]
    fn runes(&self) -> &RuneSet {
        &self.runes
    }

    /// Get mutable access to the runes information when the "runes" feature is enabled
    #[cfg(feature = "runes")]
    fn runes_mut(&mut self) -> &mut RuneSet {
        &mut self.runes
    }

    /// Get access to the consolidation information when the "utxo-consolidation" feature is enabled
    #[cfg(feature = "utxo-consolidation")]
    fn needs_consolidation(&self) -> &FixedOptionF64 {
        &self.needs_consolidation
    }

    /// Get mutable access to the consolidation information when the "utxo-consolidation" feature is enabled
    #[cfg(feature = "utxo-consolidation")]
    fn needs_consolidation_mut(&mut self) -> &mut FixedOptionF64 {
        &mut self.needs_consolidation
    }
}

// Safety: All generic parameters must also be Pod/Zeroable.
unsafe impl<RuneSet: FixedCapacitySet<Item = RuneAmount> + Pod> Pod for UtxoInfo<RuneSet> {}
unsafe impl<RuneSet: FixedCapacitySet<Item = RuneAmount> + Zeroable> Zeroable
    for UtxoInfo<RuneSet>
{
}

impl<RuneSet: FixedCapacitySet<Item = RuneAmount>> PartialEq for UtxoInfo<RuneSet> {
    fn eq(&self, other: &Self) -> bool {
        self.meta == other.meta
    }
}

impl<RuneSet: FixedCapacitySet<Item = RuneAmount>> Eq for UtxoInfo<RuneSet> {}

impl<RuneSet: FixedCapacitySet<Item = RuneAmount>> std::fmt::Display for UtxoInfo<RuneSet> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", hex::encode(&self.meta.txid()), self.meta.vout())
    }
}

impl<RuneSet: FixedCapacitySet<Item = RuneAmount>> AsRef<UtxoInfo<RuneSet>> for UtxoInfo<RuneSet> {
    fn as_ref(&self) -> &UtxoInfo<RuneSet> {
        self
    }
}

impl<RuneSet: FixedCapacitySet<Item = RuneAmount>> AsRef<UtxoMeta> for UtxoInfo<RuneSet> {
    fn as_ref(&self) -> &UtxoMeta {
        &self.meta
    }
}

impl<RuneSet: FixedCapacitySet<Item = RuneAmount>> Default for UtxoInfo<RuneSet>
where
    RuneSet: Default,
{
    fn default() -> Self {
        Self {
            meta: UtxoMeta::from([0; 32], 0),
            value: u64::default(),
            #[cfg(feature = "runes")]
            runes: RuneSet::default(),
            #[cfg(feature = "utxo-consolidation")]
            needs_consolidation: FixedOptionF64::default(),
            // Ensure the generic parameter is referenced even when the `runes` feature is disabled.
            #[cfg(not(feature = "runes"))]
            _phantom: std::marker::PhantomData::<RuneSet>,
        }
    }
}

#[cfg(feature = "runes")]
impl<RS> TryFrom<&UtxoMeta> for UtxoInfo<RS>
where
    RS: FixedCapacitySet<Item = RuneAmount> + Default,
{
    type Error = ProgramError;

    fn try_from(value: &UtxoMeta) -> std::result::Result<Self, ProgramError> {
        // Fetch rune amount (at most one) from the UTXO.
        let runes = get_runes(value)?;

        let outpoint = value.to_outpoint();

        let ui_value =
            get_bitcoin_tx_output_value(txid_to_bytes_big_endian(&outpoint.txid), outpoint.vout)
                .ok_or(BitcoinTxError::TransactionNotFound)?;

        Ok(UtxoInfo {
            meta: value.clone(),
            value: ui_value,
            runes: runes,
            #[cfg(feature = "utxo-consolidation")]
            needs_consolidation: FixedOptionF64::none(),
        })
    }
}

// When the "runes" feature is disabled, fallback implementation without rune handling.
#[cfg(not(feature = "runes"))]
impl TryFrom<&UtxoMeta> for UtxoInfo<SingleRuneSet> {
    type Error = ProgramError;

    fn try_from(value: &UtxoMeta) -> std::result::Result<Self, ProgramError> {
        let outpoint = value.to_outpoint();

        let ui_value =
            get_bitcoin_tx_output_value(txid_to_bytes_big_endian(&outpoint.txid), outpoint.vout)
                .ok_or(ProgramError::Custom(
                    BitcoinTxError::TransactionNotFound.into(),
                ))?;

        Ok(UtxoInfo {
            meta: value.clone(),
            value: ui_value,
            #[cfg(feature = "utxo-consolidation")]
            needs_consolidation: FixedOptionF64::none(),
            _phantom: std::marker::PhantomData::<SingleRuneSet>,
        })
    }
}

// === Default helper types ==================================================
// For the most common case – a single-rune set (SingleRuneSet) and up to 50
// plain-BTC UTXOs per account – we provide ready-made fixed-capacity array and
// fixed-option wrappers so downstream crates can simply use
// `FixedArrayUtxoInfo` / `FixedOptionUtxoInfo` without additional boilerplate.

declare_fixed_array!(FixedArrayUtxoInfo, UtxoInfo<SingleRuneSet>, 50);
declare_fixed_option!(FixedOptionUtxoInfo, UtxoInfo<SingleRuneSet>, 15);

// Add helper methods to validate runes contained in a UTXO
#[cfg(feature = "runes")]
impl<RuneSet> UtxoInfo<RuneSet>
where
    RuneSet: FixedCapacitySet<Item = RuneAmount>,
{
    /// Returns the number of `RuneAmount` entries stored in this UTXO.
    ///
    /// NOTE: With the default `SingleRuneSet` this will be either **0** or **1**.
    pub fn rune_entry_count(&self) -> usize {
        self.runes.len()
    }

    /// Returns the total amount of runes across **all** [`RuneAmount`] entries
    /// stored in this UTXO.
    ///
    /// When `SingleRuneSet` is used this is equivalent to `self.rune_amount().unwrap_or(0)`.
    pub fn total_rune_amount(&self) -> u128 {
        self.runes.iter().map(|r| r.amount).sum()
    }

    /// If the UTXO contains a [`RuneAmount`] with the given [`RuneId`], returns
    /// the amount, otherwise `None`.
    pub fn rune_amount(&self, rune_id: &RuneId) -> Option<u128> {
        self.runes.find(rune_id).map(|r| r.amount)
    }

    /// Convenience check that this UTXO holds **exactly** `amount` of the rune
    /// identified by `rune_id`.
    pub fn contains_exact_rune(&self, rune_id: &RuneId, amount: u128) -> bool {
        self.rune_amount(rune_id) == Some(amount)
    }
}

#[cfg(not(feature = "runes"))]
impl<RuneSet> UtxoInfo<RuneSet>
where
    RuneSet: FixedCapacitySet<Item = RuneAmount>,
{
    /// Returns zero because rune information is unavailable when the `runes` feature is disabled.
    pub fn rune_entry_count(&self) -> usize {
        0
    }

    /// Returns zero because rune information is unavailable when the `runes` feature is disabled.
    pub fn total_rune_amount(&self) -> u128 {
        0
    }

    /// Always returns `None` because rune information is unavailable when the `runes` feature is disabled.
    pub fn rune_amount(&self, _rune_id: &RuneId) -> Option<u128> {
        None
    }

    /// Always returns `false` because rune information is unavailable when the `runes` feature is disabled.
    pub fn contains_exact_rune(&self, _rune_id: &RuneId, _amount: u128) -> bool {
        false
    }
}
