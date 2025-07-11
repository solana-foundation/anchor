//! Utilities to **apply the effects of a broadcast Bitcoin transaction** to
//! a program’s shard accounts.
//!
//! While `split.rs` is concerned with *planning* the redistribution of value
//! (and extends the `TransactionBuilder` accordingly), **this module performs
//! the *state mutation***: it removes spent UTXOs, inserts newly created ones
//! and keeps Rune pointers up-to-date.
//!
//! Intended usage
//! --------------
//! 1. Build and sign a transaction with
//!    [`satellite_bitcoin::TransactionBuilder`].
//! 2. Call [`ShardSet::update_shards_after_transaction`] – a convenience wrapper
//!    around [`update_shards_after_transaction`] – passing the same
//!    `TransactionBuilder`, a `ShardSet` in the *Selected* state and the
//!    program’s redeem script.
//! 3. Once the function returns, simply let the `AccountLoader` borrows drop so
//!    that Anchor can persist the mutated accounts.
//!
//! Feature flags
//! -------------
//! * `runes` – enables Rune-aware logic (runestone edicts, pointer updates).
//! * `utxo-consolidation` – sets the consolidation flag on large UTXO sets so
//!   they can be merged in a follow-up instruction.
//!
//! Error handling
//! --------------
//! * Overflows when inserting new BTC UTXOs ⇒ `StateShardError::VecOverflow`
//! * Rune bookkeeping errors ⇒ the respective `StateShardError::*` variant.
//! * Too many BTC UTXOs for the selected shards ⇒ `StateShardError::ShardsAreFullOfBtcUtxos`
//!
//! All helpers operate only on `shard_set.selected_indices()`; unrelated shards
//! remain untouched, which allows callers to pass references to the entire
//! shards slice without cloning.

use arch_program::{input_to_sign::InputToSign, rune::RuneAmount, utxo::UtxoMeta};
use bitcoin::{ScriptBuf, Transaction};
use satellite_bitcoin::utxo_info::UtxoInfoTrait;
use satellite_bitcoin::{fee_rate::FeeRate, TransactionBuilder};

#[cfg(feature = "runes")]
use arch_program::rune::RuneId;
#[cfg(feature = "runes")]
use ordinals::Runestone;
use satellite_bitcoin::generic::fixed_set::FixedCapacitySet;

use crate::error::{Error, ErrorCode};

use super::{Selected as ShardSetSelected, ShardSet, StateShard};

use anchor_lang::prelude::Owner;
use anchor_lang::ZeroCopy;

/// Removes all `utxos_to_remove` from the shards identified by `shard_indexes`.
///
/// This is an internal helper; it assumes that each entry in
/// `utxos_to_remove` **must** be present in every shard listed in
/// `shard_indexes` (either as a BTC-UTXO or the optional rune-UTXO) and will
/// silently ignore shards where the UTXO is missing – this is fine because the
/// outer logic only passes in shards that are actually affected.
fn remove_utxos_from_shards<'slice, 'info, RS, U, S, const MAX_SEL: usize>(
    shard_set: &ShardSet<'slice, 'info, S, MAX_SEL, ShardSetSelected>,
    shard_indexes: &[usize],
    utxos_to_remove: &[UtxoMeta],
) -> crate::Result<()>
where
    RS: FixedCapacitySet<Item = RuneAmount> + Default,
    U: UtxoInfoTrait<RS>,
    S: StateShard<U, RS> + ZeroCopy + Owner,
{
    for utxo_to_remove in utxos_to_remove {
        for &idx in shard_indexes {
            let handle = shard_set.handle_by_index(idx);
            // Ignore ProgramError – treat it as a fatal StateShardError.
            handle
                .with_mut(|shard| {
                    shard.btc_utxos_retain(&mut |utxo| utxo.meta() != utxo_to_remove);

                    if let Some(rune_utxo) = shard.rune_utxo() {
                        if rune_utxo.meta() == utxo_to_remove {
                            shard.clear_rune_utxo();
                        }
                    }
                })
                .map_err(|_| Error::from(ErrorCode::RuneAmountAdditionOverflow))?;
        }
    }
    Ok(())
}

/// Selects the shard (by **global** index) with the smallest total BTC value
/// **and** spare capacity for another BTC-UTXO.
fn select_best_shard_to_add_btc_to<'slice, 'info, RS, U, S, const MAX_SEL: usize>(
    shard_set: &ShardSet<'slice, 'info, S, MAX_SEL, ShardSetSelected>,
    shard_indexes: &[usize],
) -> Option<usize>
where
    RS: FixedCapacitySet<Item = RuneAmount> + Default,
    U: UtxoInfoTrait<RS>,
    S: StateShard<U, RS> + ZeroCopy + Owner,
{
    let mut best_idx: Option<usize> = None;
    let mut smallest_total: u64 = u64::MAX;

    for &idx in shard_indexes {
        let handle = shard_set.handle_by_index(idx);
        if let Ok(can_use) = handle.with_ref(|shard| {
            let spare = shard.btc_utxos_len() < shard.btc_utxos_max_len();
            let sum: u64 = shard.btc_utxos().iter().map(|u| u.value()).sum();
            (spare, sum)
        }) {
            let (spare, sum) = can_use;
            if spare && sum < smallest_total {
                smallest_total = sum;
                best_idx = Some(idx);
            }
        }
    }

    best_idx
}

/// Updates the UTXO sets of the provided shards.
#[allow(clippy::too_many_arguments)]
fn update_shards_utxos<'slice, 'info, RS, U, S, const MAX_SEL: usize>(
    shard_set: &ShardSet<'slice, 'info, S, MAX_SEL, ShardSetSelected>,
    shard_indexes: &[usize],
    utxos_to_remove: &[UtxoMeta],
    new_rune_utxos: Vec<U>,
    mut new_btc_utxos: Vec<U>,
    _fee_rate: &FeeRate,
) -> crate::Result<()>
where
    RS: FixedCapacitySet<Item = RuneAmount> + Default,
    U: UtxoInfoTrait<RS>,
    S: StateShard<U, RS> + ZeroCopy + Owner,
{
    // 1. Remove old UTXOs first.
    remove_utxos_from_shards::<RS, U, S, MAX_SEL>(shard_set, shard_indexes, utxos_to_remove)?;

    // 2. Insert rune UTXOs where needed.
    let mut rune_utxo_iter = new_rune_utxos.into_iter();
    for &shard_index in shard_indexes {
        let handle = shard_set.handle_by_index(shard_index);
        handle
            .with_mut(|shard| {
                if shard.rune_utxo().is_none() {
                    if let Some(utxo) = rune_utxo_iter.next() {
                        shard.set_rune_utxo(utxo);
                    }
                }
            })
            .map_err(|_| Error::from(ErrorCode::RuneAmountAdditionOverflow))?;
    }

    // After distribution there must be **no** leftover rune-bearing UTXOs.
    // Having some left would mean that we would lose tokens on-chain.
    if rune_utxo_iter.next().is_some() {
        return Err(ErrorCode::ExcessRuneUtxos.into());
    }

    // 3. Distribute BTC UTXOs – largest first – to the least funded shard.
    new_btc_utxos.sort_by(|a, b| b.value().cmp(&a.value()));

    for utxo in new_btc_utxos.into_iter() {
        // Select target shard.
        let target_idx =
            select_best_shard_to_add_btc_to::<RS, U, S, MAX_SEL>(shard_set, shard_indexes)
                .ok_or(Error::from(ErrorCode::ShardsAreFullOfBtcUtxos))?;

        let handle = shard_set.handle_by_index(target_idx);

        // Apply consolidation flag if feature enabled.
        #[cfg(feature = "utxo-consolidation")]
        {
            use satellite_bitcoin::utxo_info::FixedOptionF64;
            if handle
                .with_ref(|shard| shard.btc_utxos_len() > 1)
                .unwrap_or(false)
            {
                *utxo.needs_consolidation_mut() = FixedOptionF64::some(fee_rate.0);
            }
        }

        let success = handle
            .with_mut(|shard| shard.add_btc_utxo(utxo).is_some())
            .map_err(|_| Error::from(ErrorCode::ShardsAreFullOfBtcUtxos))?;

        if !success {
            return Err(ErrorCode::ShardsAreFullOfBtcUtxos.into());
        }
    }

    Ok(())
}

/// Updates the provided `shards` to reflect the effects of a transaction that
/// has just been **broadcast and accepted**.
///
/// The function performs three high-level steps:
/// 1. Determine which program-owned UTXOs were **spent** and which new ones were
///    **created** by looking at the `TransactionBuilder` and the final
///    `transaction` that was signed.
/// 2. Split the newly created outputs into *plain BTC* vs *rune carrying*
///    outputs (the latter is only compiled in when the `runes` feature is
///    enabled).
/// 3. Call an internal balancing helper so that the new UTXOs are evenly
///    distributed across the shards involved in the call.
///
/// Only the shards contained in `shard_set.selected_indices()` are mutated,
/// allowing callers to pass references to the *entire* shards slice without
/// cloning or allocating temporaries.
///
/// # Errors
/// Returns `StateShardError::ShardsAreFullOfBtcUtxos` when all involved shards
/// have reached their fixed-size BTC-UTXO capacity.
#[allow(clippy::too_many_arguments)]
pub fn update_shards_after_transaction<
    'slice,
    'info,
    const MAX_USER_UTXOS: usize,
    const MAX_SHARDS_PER_PROGRAM: usize,
    const MAX_SEL: usize,
    RS,
    U,
    S,
>(
    transaction_builder: &mut TransactionBuilder<MAX_USER_UTXOS, MAX_SHARDS_PER_PROGRAM, RS>,
    shard_set: &ShardSet<'slice, 'info, S, MAX_SEL, ShardSetSelected>,
    program_script_pubkey: &ScriptBuf,
    fee_rate: &FeeRate,
) -> crate::Result<()>
where
    RS: FixedCapacitySet<Item = RuneAmount> + Default,
    U: UtxoInfoTrait<RS>,
    S: StateShard<U, RS> + ZeroCopy + Owner,
{
    // ---------------------------------------------------------------------
    // 1. Identify program-owned UTXOs that were spent/created.
    // ---------------------------------------------------------------------
    let (utxos_to_remove, new_program_utxos) = get_modified_program_utxos_in_transaction::<RS, U>(
        program_script_pubkey,
        &transaction_builder.transaction,
        transaction_builder.inputs_to_sign.as_slice(),
    );

    // ---------------------------------------------------------------------
    // 2. Split new outputs into rune vs btc (feature-gated like original).
    // ---------------------------------------------------------------------
    #[cfg(feature = "runes")]
    let (new_rune_utxos, new_btc_utxos) = {
        let runestone = &transaction_builder.runestone;

        let new_rune_utxos = update_modified_program_utxos_with_rune_amount::<RS, U>(
            &mut new_program_utxos,
            runestone,
            &mut transaction_builder.total_rune_inputs,
        )?;
        (new_rune_utxos, new_program_utxos)
    };

    #[cfg(not(feature = "runes"))]
    let (new_rune_utxos, new_btc_utxos) = (Vec::<U>::new(), new_program_utxos);

    // ---------------------------------------------------------------------
    // 3. Mutate shards.
    // ---------------------------------------------------------------------
    let shard_indexes = shard_set.selected_indices();
    update_shards_utxos::<RS, U, S, MAX_SEL>(
        shard_set,
        shard_indexes,
        &utxos_to_remove,
        new_rune_utxos,
        new_btc_utxos,
        fee_rate,
    )
}

fn get_modified_program_utxos_in_transaction<RS, U>(
    program_script_pubkey: &ScriptBuf,
    transaction: &Transaction,
    inputs_to_sign: &[InputToSign],
) -> (Vec<UtxoMeta>, Vec<U>)
where
    RS: FixedCapacitySet<Item = RuneAmount> + Default,
    U: UtxoInfoTrait<RS>,
{
    use satellite_bitcoin::bytes::txid_to_bytes_big_endian;

    let mut utxos_to_remove = Vec::with_capacity(inputs_to_sign.len());
    let mut program_outputs = Vec::with_capacity(transaction.output.len() / 2);

    let txid_bytes = txid_to_bytes_big_endian(&transaction.compute_txid());

    for input in inputs_to_sign {
        let outpoint = transaction.input[input.index as usize].previous_output;
        utxos_to_remove.push(UtxoMeta::from(
            txid_to_bytes_big_endian(&outpoint.txid),
            outpoint.vout,
        ));
    }

    for (index, output) in transaction.output.iter().enumerate() {
        if &output.script_pubkey == program_script_pubkey {
            program_outputs.push(U::new(
                UtxoMeta::from(txid_bytes, index as u32),
                output.value.to_sat(),
            ));
        }
    }

    (utxos_to_remove, program_outputs)
}

#[cfg(feature = "runes")]
fn update_modified_program_utxos_with_rune_amount<RS, U>(
    new_program_outputs: &mut Vec<U>,
    runestone: &Runestone,
    prev_rune_amount: &mut RS,
) -> Result<Vec<U>>
where
    RS: FixedCapacitySet<Item = RuneAmount> + Default,
    U: UtxoInfoTrait<RS>,
{
    let remaining_rune_amount = prev_rune_amount;
    let mut rune_utxos = vec![];

    for edict in &runestone.edicts {
        let rune_amount = edict.amount;
        let index = edict.output;
        let pos = new_program_outputs
            .iter()
            .position(|u| u.meta().vout() == index)
            .ok_or(StateShardError::OutputEdictIsNotInTransaction)?;

        let output = new_program_outputs
            .get_mut(pos)
            .ok_or(StateShardError::OutputEdictIsNotInTransaction)?;

        let rune_id = RuneId::new(edict.id.block, edict.id.tx);

        output.runes_mut().insert_or_modify::<StateShardError, _>(
            RuneAmount {
                id: rune_id,
                amount: rune_amount,
            },
            |rune_input| {
                rune_input.amount = rune_input
                    .amount
                    .checked_add(rune_amount)
                    .ok_or(StateShardError::RuneAmountAdditionOverflow)?;
                Ok(())
            },
        )?;

        if let Some(remaining) = remaining_rune_amount.find_mut(&rune_id) {
            remaining.amount = remaining
                .amount
                .checked_sub(rune_amount)
                .ok_or(StateShardError::NotEnoughRuneInShards)?;
        }
    }

    if let Some(pointer_index) = runestone.pointer {
        for rune_amount in remaining_rune_amount.iter() {
            if rune_amount.amount > 0 {
                if let Some(output) = new_program_outputs
                    .iter_mut()
                    .find(|u| u.meta().vout() == pointer_index)
                {
                    output.runes_mut().insert_or_modify::<StateShardError, _>(
                        RuneAmount {
                            id: rune_amount.id,
                            amount: rune_amount.amount,
                        },
                        |rune_input| {
                            rune_input.amount =
                                rune_input
                                    .amount
                                    .checked_add(rune_amount.amount)
                                    .ok_or(StateShardError::RuneAmountAdditionOverflow)?;
                            Ok(())
                        },
                    )?;
                } else {
                    return Err(StateShardError::RunestonePointerIsNotInTransaction);
                }
            }
        }
    } else {
        for rune_amount in remaining_rune_amount.iter() {
            if rune_amount.amount > 0 {
                return Err(StateShardError::RunestonePointerIsNotInTransaction);
            }
        }
    }

    let mut i = new_program_outputs.len();
    while i > 0 {
        i -= 1;
        if new_program_outputs[i].runes().len() > 0 {
            let rune_utxo = new_program_outputs.swap_remove(i);
            rune_utxos.push(rune_utxo);
        }
    }

    rune_utxos.reverse();

    Ok(rune_utxos)
}

#[cfg(test)]
mod tests_loader {
    use super::*;

    use crate::shard_set::tests::common::{
        add_btc_utxos_bulk, create_shard, leak_loaders_from_vec, MockShardZc, MAX_BTC_UTXOS,
    };
    use crate::shard_set::ShardSet;
    use satellite_bitcoin::utxo_info::{SingleRuneSet, UtxoInfo, UtxoInfoTrait};

    // Re-export for macro reuse – mirrors helper in split_loader tests.
    use satellite_bitcoin::TransactionBuilder as TB;

    #[allow(unused_macros)]
    macro_rules! new_tb {
        ($max_utxos:expr, $max_shards:expr) => {
            TB::<$max_utxos, $max_shards, SingleRuneSet>::new()
        };
    }

    // === Shared helpers ====================================================
    fn create_utxo(
        value: u64,
        txid_byte: u8,
        vout: u32,
    ) -> satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet> {
        let txid = [txid_byte; 32];
        let meta = UtxoMeta::from(txid, vout);
        let utxo_info = UtxoInfo::new(meta, value);
        utxo_info
    }

    fn fee_rate() -> FeeRate {
        FeeRate(1.0)
    }

    // ---------------------------------------------------------------------
    // select_best_shard_to_add_btc_to
    // ---------------------------------------------------------------------
    mod select_best_shard_to_add_btc_to {
        use super::*;

        #[test]
        fn selects_shard_with_smallest_total_btc() {
            let shard_low = create_shard(50);
            let shard_medium = create_shard(100);
            let shard_high = create_shard(200);

            let shards_vec = vec![shard_medium, shard_low, shard_high];
            let loaders = leak_loaders_from_vec(shards_vec);
            const MAX_SEL: usize = 3;
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let selected = unselected.select_with([0usize, 1, 2]).unwrap();

            let best = super::super::select_best_shard_to_add_btc_to::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(&selected, selected.selected_indices());

            assert_eq!(best, Some(1)); // shard_low at index 1 has the fewest sats
        }

        #[test]
        fn returns_none_when_all_shards_are_full() {
            let mut shard0 = create_shard(0);
            let mut shard1 = create_shard(0);
            // Fill both shards to capacity
            add_btc_utxos_bulk(&mut shard0, &vec![1u64; MAX_BTC_UTXOS]);
            add_btc_utxos_bulk(&mut shard1, &vec![1u64; MAX_BTC_UTXOS]);

            let shards_vec = vec![shard0, shard1];
            let loaders = leak_loaders_from_vec(shards_vec);
            const MAX_SEL: usize = 2;
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let selected = unselected.select_with([0usize, 1]).unwrap();

            let res = super::super::select_best_shard_to_add_btc_to::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(&selected, selected.selected_indices());
            assert_eq!(res, None);
        }

        #[test]
        fn skips_full_shard_and_selects_available_one() {
            let mut shard_full = create_shard(0);
            add_btc_utxos_bulk(&mut shard_full, &vec![1u64; MAX_BTC_UTXOS]);
            let shard_available = create_shard(500);

            let shards_vec = vec![shard_full, shard_available];
            let loaders = leak_loaders_from_vec(shards_vec);
            const MAX_SEL: usize = 2;
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let selected = unselected.select_with([0usize, 1]).unwrap();

            let res = super::super::select_best_shard_to_add_btc_to::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(&selected, selected.selected_indices());
            assert_eq!(res, Some(1)); // second shard has spare capacity
        }
    }

    // ---------------------------------------------------------------------
    // update_shards_utxos (subset)
    // ---------------------------------------------------------------------
    mod update_shards_utxos_tests {
        use super::*;

        const MAX_SEL: usize = 2;

        fn setup_shard_set(
            shard0: MockShardZc,
            shard1: MockShardZc,
        ) -> ShardSet<'static, 'static, MockShardZc, MAX_SEL, crate::shard_set::Selected> {
            let shards_vec = vec![shard0, shard1];
            let loaders = leak_loaders_from_vec(shards_vec);
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            unselected.select_with([0usize, 1]).unwrap()
        }

        #[test]
        fn distributes_new_utxos_and_handles_runes() {
            let shard_set = setup_shard_set(create_shard(0), create_shard(0));
            let shard_indexes = shard_set.selected_indices();

            let new_rune_utxo = create_utxo(546, 10, 0);
            let new_btc_big = create_utxo(200, 11, 0);
            let new_btc_small = create_utxo(100, 12, 0);

            let result = super::super::update_shards_utxos::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(
                &shard_set,
                shard_indexes,
                &[],
                vec![new_rune_utxo.clone()],
                vec![new_btc_big.clone(), new_btc_small.clone()],
                &fee_rate(),
            );
            assert!(result.is_ok());

            // Verify shard0 (index 0) received rune utxo and larger btc value
            let handle0 = shard_set.handle_by_index(0);
            let shard0_btc_len = handle0.with_ref(|s| s.btc_utxos_len()).unwrap();
            let shard0_rune_present = handle0.with_ref(|s| s.rune_utxo().is_some()).unwrap();
            assert_eq!(shard0_btc_len, 1);
            assert!(shard0_rune_present);

            // shard1 should have smaller btc and no rune
            let handle1 = shard_set.handle_by_index(1);
            let shard1_btc_len = handle1.with_ref(|s| s.btc_utxos_len()).unwrap();
            let shard1_rune_present = handle1.with_ref(|s| s.rune_utxo().is_some()).unwrap();
            assert_eq!(shard1_btc_len, 1);
            assert!(!shard1_rune_present);
        }

        #[test]
        fn errors_when_btc_utxo_vector_overflows() {
            // Fill both shards
            let mut shard0 = create_shard(0);
            add_btc_utxos_bulk(&mut shard0, &vec![1u64; MAX_BTC_UTXOS]);
            let mut shard1 = create_shard(0);
            add_btc_utxos_bulk(&mut shard1, &vec![1u64; MAX_BTC_UTXOS]);

            let shard_set = setup_shard_set(shard0, shard1);
            let shard_indexes = shard_set.selected_indices();

            let err = super::super::update_shards_utxos::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(
                &shard_set,
                shard_indexes,
                &[],
                vec![],
                vec![create_utxo(1, 99, 0)],
                &fee_rate(),
            )
            .unwrap_err();

            assert_eq!(err, ErrorCode::ShardsAreFullOfBtcUtxos.into());
        }

        #[test]
        fn succeeds_after_removal_creates_capacity() {
            // Fill shard0 to capacity (MAX_BTC_UTXOS) and shard1 empty.
            let mut shard0 = MockShardZc::default();

            // First UTXO that we will remove.
            let utxo_to_remove = create_utxo(100, 120, 0);
            shard0.add_btc_utxo(utxo_to_remove.clone());

            // Fill rest of shard0
            let filler: Vec<u64> = vec![1u64; MAX_BTC_UTXOS - 1];
            add_btc_utxos_bulk(&mut shard0, &filler);

            let shard1 = MockShardZc::default();

            let shards_vec = vec![shard0, shard1];
            let loaders = leak_loaders_from_vec(shards_vec);
            const MAX_SEL: usize = 2;
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize, 1usize]).unwrap();

            let new_utxo = create_utxo(200, 122, 0);

            // Execute update – should succeed because removal frees 1 slot.
            super::super::update_shards_utxos::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(
                &shard_set,
                shard_set.selected_indices(),
                &[*utxo_to_remove.meta()],
                vec![],
                vec![new_utxo.clone()],
                &fee_rate(),
            )
            .unwrap();

            // shard0 should still be at capacity and no longer contain utxo_to_remove
            let h0 = shard_set.handle_by_index(0);
            h0.with_ref(|s| {
                assert_eq!(s.btc_utxos_len(), MAX_BTC_UTXOS - 1);
                assert!(!s.btc_utxos().iter().any(|u| u.eq_meta(&utxo_to_remove)));
            })
            .unwrap();

            // shard1 should now contain the new_utxo (least funded after removal)
            let h1 = shard_set.handle_by_index(1);
            h1.with_ref(|s| {
                assert_eq!(s.btc_utxos_len(), 1);
                assert!(s.btc_utxos().iter().any(|u| u.eq_meta(&new_utxo)));
            })
            .unwrap();
        }

        #[test]
        fn replaces_rune_utxo_correctly() {
            let old_rune = create_utxo(546, 130, 0);
            let new_rune = create_utxo(546, 131, 0);

            let mut shard0 = MockShardZc::default();
            shard0.set_rune_utxo(old_rune.clone());

            let shard1 = MockShardZc::default();

            let loaders = leak_loaders_from_vec(vec![shard0, shard1]);
            const MAX_SEL: usize = 2;
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize, 1usize]).unwrap();

            super::super::update_shards_utxos::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(
                &shard_set,
                shard_set.selected_indices(),
                &[*old_rune.meta()],
                vec![new_rune.clone()],
                vec![],
                &fee_rate(),
            )
            .unwrap();

            let h0 = shard_set.handle_by_index(0);
            h0.with_ref(|s| {
                let r = s.rune_utxo().expect("rune utxo expected");
                assert!(r.eq_meta(&new_rune));
            })
            .unwrap();

            let h1 = shard_set.handle_by_index(1);
            h1.with_ref(|s| assert!(s.rune_utxo().is_none())).unwrap();
        }

        #[cfg(feature = "utxo-consolidation")]
        #[test]
        fn sets_needs_consolidation_flag_when_applicable() {
            // shard0 has 2 tiny UTXOs so will receive the new one
            let mut shard0 = MockShardZc::default();
            add_btc_utxos_bulk(&mut shard0, &[1, 1]);

            let mut shard1 = MockShardZc::default();
            add_btc_utxos_bulk(&mut shard1, &[100]);

            let loaders = leak_loaders_from_vec(vec![shard0, shard1]);
            const MAX_SEL: usize = 2;
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize, 1usize]).unwrap();

            let new_utxo = create_utxo(5, 83, 0);

            super::super::update_shards_utxos::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(
                &shard_set,
                shard_set.selected_indices(),
                &[],
                vec![],
                vec![new_utxo.clone()],
                &fee_rate(),
            )
            .unwrap();

            let h0 = shard_set.handle_by_index(0);
            h0.with_ref(|s| {
                let inserted = s.btc_utxos().last().unwrap();
                assert!(inserted.needs_consolidation().is_some());
                assert_eq!(inserted.needs_consolidation().get().unwrap(), fee_rate().0);
            })
            .unwrap();
        }

        #[cfg(feature = "utxo-consolidation")]
        #[test]
        fn does_not_set_consolidation_flag_when_shard_has_one_or_zero_utxos() {
            let mut shard0 = MockShardZc::default();
            add_btc_utxos_bulk(&mut shard0, &[50]);

            let shard1 = MockShardZc::default();

            let loaders = leak_loaders_from_vec(vec![shard0, shard1]);
            const MAX_SEL: usize = 2;
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize, 1usize]).unwrap();

            let new_utxo = create_utxo(10, 151, 0);

            super::super::update_shards_utxos::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(
                &shard_set,
                shard_set.selected_indices(),
                &[],
                vec![],
                vec![new_utxo.clone()],
                &fee_rate(),
            )
            .unwrap();

            // new UTXO should go to shard1 (empty before)
            let h1 = shard_set.handle_by_index(1);
            h1.with_ref(|s| {
                assert_eq!(s.btc_utxos_len(), 1);
                let inserted = s.btc_utxos().first().unwrap();
                assert!(inserted.needs_consolidation().is_none());
            })
            .unwrap();
        }

        #[test]
        fn skips_inserting_rune_when_already_present() {
            const MAX_SEL: usize = 2;
            // shard0 already has a rune UTXO
            let existing_rune = create_utxo(546, 30, 0);
            let mut shard0 = MockShardZc::default();
            shard0.set_rune_utxo(existing_rune.clone());

            let shard1 = MockShardZc::default();

            let loaders = leak_loaders_from_vec(vec![shard0, shard1]);
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize, 1usize]).unwrap();

            // Attempt to insert a new rune UTXO – should go to shard1, not replace shard0's
            let new_rune = create_utxo(546, 31, 0);

            super::super::update_shards_utxos::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(
                &shard_set,
                shard_set.selected_indices(),
                &[],
                vec![new_rune.clone()],
                vec![],
                &fee_rate(),
            )
            .unwrap();

            // Verify shard0 still has original rune
            shard_set
                .handle_by_index(0)
                .with_ref(|s| assert!(s.rune_utxo().unwrap().eq_meta(&existing_rune)))
                .unwrap();

            // shard1 received new rune
            shard_set
                .handle_by_index(1)
                .with_ref(|s| {
                    assert!(s.rune_utxo().is_some());
                    assert!(s.rune_utxo().unwrap().eq_meta(&new_rune));
                })
                .unwrap();
        }

        #[test]
        fn handles_no_new_runes_when_shards_have_none() {
            const MAX_SEL: usize = 2;
            let shard0 = MockShardZc::default();
            let shard1 = MockShardZc::default();
            let loaders = leak_loaders_from_vec(vec![shard0, shard1]);
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize, 1usize]).unwrap();

            let btc_utxo = create_utxo(1_000, 140, 0);

            super::super::update_shards_utxos::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(
                &shard_set,
                shard_set.selected_indices(),
                &[],
                vec![],
                vec![btc_utxo],
                &fee_rate(),
            )
            .unwrap();

            // Neither shard should have a rune utxo.
            for &idx in shard_set.selected_indices() {
                shard_set
                    .handle_by_index(idx)
                    .with_ref(|s| assert!(s.rune_utxo().is_none()))
                    .unwrap();
            }
        }
    }

    // ---------------------------------------------------------------------
    // remove_utxos_from_shards
    // ---------------------------------------------------------------------
    mod remove_utxos_from_shards {
        use super::*;
        const MAX_SEL: usize = 2;

        #[test]
        fn removes_btc_and_rune_utxos_across_shards() {
            // UTXO to be removed
            let utxo_to_remove = create_utxo(1_000, 200, 0);
            let meta_to_remove = *utxo_to_remove.meta();

            // Build two shards each containing the BTC + Rune UTXO to remove
            let mut shard0 = MockShardZc::default();
            shard0.add_btc_utxo(utxo_to_remove.clone());
            shard0.set_rune_utxo(utxo_to_remove.clone());

            let mut shard1 = MockShardZc::default();
            shard1.add_btc_utxo(utxo_to_remove.clone());
            shard1.set_rune_utxo(utxo_to_remove.clone());

            // Create ShardSet holding both shards
            let shards_vec = vec![shard0, shard1];
            let loaders = leak_loaders_from_vec(shards_vec);
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize, 1usize]).unwrap();
            let idxs = shard_set.selected_indices();

            // Execute helper and verify
            super::super::remove_utxos_from_shards::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(&shard_set, idxs, &[meta_to_remove])
            .unwrap();

            for &idx in idxs {
                let h = shard_set.handle_by_index(idx);
                h.with_ref(|s| {
                    assert_eq!(s.btc_utxos_len(), 0);
                    assert!(s.rune_utxo().is_none());
                })
                .unwrap();
            }
        }

        #[test]
        fn ignores_utxo_missing_in_some_shards() {
            let utxo_to_remove = create_utxo(500, 201, 0);
            let meta_to_remove = *utxo_to_remove.meta();

            // shard0 contains the UTXO, shard1 does not
            let mut shard0 = MockShardZc::default();
            shard0.add_btc_utxo(utxo_to_remove.clone());

            let shard1 = MockShardZc::default();

            let shards_vec = vec![shard0, shard1];
            let loaders = leak_loaders_from_vec(shards_vec);
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize, 1usize]).unwrap();
            let idxs = shard_set.selected_indices();

            super::super::remove_utxos_from_shards::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(&shard_set, idxs, &[meta_to_remove])
            .unwrap();

            // shard0 should now be empty, shard1 unaffected
            let h0 = shard_set.handle_by_index(0);
            h0.with_ref(|s| assert_eq!(s.btc_utxos_len(), 0)).unwrap();
            let h1 = shard_set.handle_by_index(1);
            h1.with_ref(|s| assert_eq!(s.btc_utxos_len(), 0)).unwrap();
        }

        #[test]
        fn handles_empty_utxos_to_remove() {
            let shard_set = {
                let shard0 = create_shard(1000);
                let shard1 = create_shard(2000);
                let loaders = leak_loaders_from_vec(vec![shard0, shard1]);
                let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
                unselected.select_with([0usize, 1usize]).unwrap()
            };

            let idxs = shard_set.selected_indices();
            // Removing zero items should be a no-op
            super::super::remove_utxos_from_shards::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(&shard_set, idxs, &[])
            .unwrap();

            // Verify original balances intact
            shard_set
                .handle_by_index(0)
                .with_ref(|s| assert_eq!(s.btc_utxos_len(), 1))
                .unwrap();
            shard_set
                .handle_by_index(1)
                .with_ref(|s| assert_eq!(s.btc_utxos_len(), 1))
                .unwrap();
        }

        #[test]
        fn works_when_shard_has_no_rune_utxo() {
            const MAX_SEL: usize = 1;
            let utxo_to_remove = create_utxo(1_000, 60, 0);
            let meta = *utxo_to_remove.meta();

            let mut shard = MockShardZc::default();
            shard.add_btc_utxo(utxo_to_remove.clone());

            let loaders = leak_loaders_from_vec(vec![shard]);
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize]).unwrap();

            super::super::remove_utxos_from_shards::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(&shard_set, shard_set.selected_indices(), &[meta])
            .unwrap();

            shard_set
                .handle_by_index(0)
                .with_ref(|s| assert_eq!(s.btc_utxos_len(), 0))
                .unwrap();
        }

        #[test]
        fn removes_multiple_utxos_from_multiple_shards() {
            const MAX_SEL: usize = 2;
            let utxo_a = create_utxo(500, 250, 0);
            let utxo_b = create_utxo(600, 251, 0);

            let mut shard0 = MockShardZc::default();
            shard0.add_btc_utxo(utxo_a.clone());
            shard0.add_btc_utxo(utxo_b.clone());

            let mut shard1 = MockShardZc::default();
            shard1.add_btc_utxo(utxo_a.clone());
            shard1.add_btc_utxo(utxo_b.clone());

            let loaders = leak_loaders_from_vec(vec![shard0, shard1]);
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize, 1usize]).unwrap();

            super::super::remove_utxos_from_shards::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
                MAX_SEL,
            >(
                &shard_set,
                shard_set.selected_indices(),
                &[*utxo_a.meta(), *utxo_b.meta()],
            )
            .unwrap();

            for &idx in shard_set.selected_indices() {
                shard_set
                    .handle_by_index(idx)
                    .with_ref(|s| assert_eq!(s.btc_utxos_len(), 0))
                    .unwrap();
            }
        }
    }

    // ---------------------------------------------------------------------
    // get_modified_program_utxos_in_transaction
    // ---------------------------------------------------------------------
    mod get_modified_program_utxos_in_transaction {
        use super::*;
        use arch_program::input_to_sign::InputToSign;
        use bitcoin::absolute::LockTime;
        use bitcoin::transaction::Version;
        use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};

        #[test]
        fn identifies_program_outputs_correctly() {
            let script = ScriptBuf::new();

            let tx = Transaction {
                version: Version::TWO,
                lock_time: LockTime::ZERO,
                input: vec![TxIn {
                    previous_output: OutPoint::null(),
                    script_sig: ScriptBuf::new(),
                    sequence: Sequence::MAX,
                    witness: Witness::default(),
                }],
                output: vec![TxOut {
                    value: Amount::from_sat(1000),
                    script_pubkey: script.clone(),
                }],
            };

            let inputs = vec![InputToSign {
                index: 0,
                signer: arch_program::pubkey::Pubkey::default(),
            }];

            let (removed, added): (
                Vec<UtxoMeta>,
                Vec<satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>>,
            ) = super::super::get_modified_program_utxos_in_transaction::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
            >(&script, &tx, &inputs);

            assert_eq!(removed.len(), 1);
            assert_eq!(added.len(), 1);
            assert_eq!(added[0].value, 1000);
        }

        #[test]
        fn handles_multiple_inputs_to_sign() {
            let script = ScriptBuf::new();

            let outpoint1 = {
                let mut o = OutPoint::null();
                o.vout = 0;
                o
            };
            let outpoint2 = {
                let mut o = OutPoint::null();
                o.vout = 1;
                o
            };

            let tx = Transaction {
                version: Version::TWO,
                lock_time: LockTime::ZERO,
                input: vec![
                    TxIn {
                        previous_output: outpoint1,
                        script_sig: ScriptBuf::new(),
                        sequence: Sequence::MAX,
                        witness: Witness::default(),
                    },
                    TxIn {
                        previous_output: outpoint2,
                        script_sig: ScriptBuf::new(),
                        sequence: Sequence::MAX,
                        witness: Witness::default(),
                    },
                ],
                output: vec![],
            };

            let inputs = vec![
                InputToSign {
                    index: 0,
                    signer: arch_program::pubkey::Pubkey::default(),
                },
                InputToSign {
                    index: 1,
                    signer: arch_program::pubkey::Pubkey::default(),
                },
            ];

            let (removed, _added): (
                Vec<UtxoMeta>,
                Vec<satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>>,
            ) = super::super::get_modified_program_utxos_in_transaction::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
            >(&script, &tx, &inputs);

            assert_eq!(removed.len(), 2);
            assert!(removed.iter().any(|m| m.vout() == 0));
            assert!(removed.iter().any(|m| m.vout() == 1));
        }

        #[test]
        fn handles_multiple_program_outputs() {
            let script = ScriptBuf::new();

            let tx = Transaction {
                version: Version::TWO,
                lock_time: LockTime::ZERO,
                input: vec![],
                output: vec![
                    TxOut {
                        value: Amount::from_sat(1_000),
                        script_pubkey: script.clone(),
                    },
                    TxOut {
                        value: Amount::from_sat(2_000),
                        script_pubkey: ScriptBuf::from_bytes(vec![0x51]),
                    },
                    TxOut {
                        value: Amount::from_sat(3_000),
                        script_pubkey: script.clone(),
                    },
                ],
            };

            let (_removed, added): (
                Vec<UtxoMeta>,
                Vec<satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>>,
            ) = super::super::get_modified_program_utxos_in_transaction::<
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
            >(&script, &tx, &[]);

            assert_eq!(added.len(), 2);
            assert_eq!(added[0].value, 1_000);
            assert_eq!(added[0].meta.vout(), 0);
            assert_eq!(added[1].value, 3_000);
            assert_eq!(added[1].meta.vout(), 2);
        }
    }

    // ---------------------------------------------------------------------
    // update_shards_after_transaction
    // ---------------------------------------------------------------------
    mod update_shards_after_transaction {
        use super::*;
        use arch_program::input_to_sign::InputToSign;
        use bitcoin::absolute::LockTime;
        use bitcoin::hashes::sha256d::Hash as Sha256dHash;
        use bitcoin::hashes::Hash;
        use bitcoin::transaction::Version;
        use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};

        #[test]
        fn integrates_all_helpers() {
            const MAX_USER_UTXOS: usize = 4;
            const MAX_SHARDS_PER_PROGRAM: usize = 4;
            const MAX_SEL: usize = 2;

            let mut builder: satellite_bitcoin::TransactionBuilder<
                MAX_USER_UTXOS,
                MAX_SHARDS_PER_PROGRAM,
                SingleRuneSet,
            > = new_tb!(MAX_USER_UTXOS, MAX_SHARDS_PER_PROGRAM);

            let program_script = ScriptBuf::new();

            // existing utxo in shard0
            let existing_utxo = create_utxo(5_000, 200, 0);
            let txid_200 =
                bitcoin::Txid::from_raw_hash(Sha256dHash::from_slice(&[200u8; 32]).unwrap());
            let input_outpoint = OutPoint {
                txid: txid_200,
                vout: 0,
            };

            builder.transaction = Transaction {
                version: Version::TWO,
                lock_time: LockTime::ZERO,
                input: vec![TxIn {
                    previous_output: input_outpoint,
                    script_sig: ScriptBuf::new(),
                    sequence: Sequence::MAX,
                    witness: Witness::default(),
                }],
                output: vec![TxOut {
                    value: Amount::from_sat(4_500),
                    script_pubkey: program_script.clone(),
                }],
            };

            builder
                .inputs_to_sign
                .push(InputToSign {
                    index: 0,
                    signer: arch_program::pubkey::Pubkey::default(),
                })
                .unwrap();

            let mut shard0 = MockShardZc::default();
            shard0.add_btc_utxo(existing_utxo.clone());
            let shard1 = MockShardZc::default();

            let loaders = leak_loaders_from_vec(vec![shard0, shard1]);
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize, 1usize]).unwrap();

            super::super::update_shards_after_transaction::<
                MAX_USER_UTXOS,
                MAX_SHARDS_PER_PROGRAM,
                MAX_SEL,
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
            >(&mut builder, &shard_set, &program_script, &fee_rate())
            .unwrap();

            // old utxo removed
            shard_set
                .handle_by_index(0)
                .with_ref(|s| assert!(!s.btc_utxos().iter().any(|u| u.eq_meta(&existing_utxo))))
                .unwrap();

            let total: usize = shard_set
                .handle_by_index(0)
                .with_ref(|s| s.btc_utxos_len())
                .unwrap()
                + shard_set
                    .handle_by_index(1)
                    .with_ref(|s| s.btc_utxos_len())
                    .unwrap();
            assert_eq!(total, 1);
        }

        #[cfg(feature = "runes")]
        #[test]
        fn handles_rune_utxo_spending_and_creation() {
            const MAX_USER_UTXOS: usize = 4;
            const MAX_SHARDS_PER_PROGRAM: usize = 4;
            const MAX_SEL: usize = 2;

            let mut builder: satellite_bitcoin::TransactionBuilder<
                MAX_USER_UTXOS,
                MAX_SHARDS_PER_PROGRAM,
                SingleRuneSet,
            > = new_tb!(MAX_USER_UTXOS, MAX_SHARDS_PER_PROGRAM);

            let program_script = ScriptBuf::new();
            let existing_rune_utxo = create_utxo(546, 210, 0);

            builder
                .total_rune_inputs
                .insert(arch_program::rune::RuneAmount {
                    id: arch_program::rune::RuneId::new(1, 0),
                    amount: 100,
                })
                .unwrap();

            let txid_210 =
                bitcoin::Txid::from_raw_hash(Sha256dHash::from_slice(&[210u8; 32]).unwrap());
            let input_outpoint = OutPoint {
                txid: txid_210,
                vout: 0,
            };

            builder.transaction = Transaction {
                version: Version::TWO,
                lock_time: LockTime::ZERO,
                input: vec![TxIn {
                    previous_output: input_outpoint,
                    script_sig: ScriptBuf::new(),
                    sequence: Sequence::MAX,
                    witness: Witness::default(),
                }],
                output: vec![
                    TxOut {
                        value: Amount::from_sat(546),
                        script_pubkey: program_script.clone(),
                    },
                    TxOut {
                        value: Amount::from_sat(546),
                        script_pubkey: program_script.clone(),
                    },
                ],
            };

            builder
                .inputs_to_sign
                .push(InputToSign {
                    index: 0,
                    signer: arch_program::pubkey::Pubkey::default(),
                })
                .unwrap();

            builder.runestone = Runestone {
                pointer: Some(1),
                edicts: vec![ordinals::Edict {
                    id: ordinals::RuneId { block: 1, tx: 0 },
                    amount: 60,
                    output: 0,
                }],
                ..Default::default()
            };

            let mut shard0 = MockShardZc::default();
            shard0.set_rune_utxo(existing_rune_utxo.clone());
            let shard1 = MockShardZc::default();

            let loaders = leak_loaders_from_vec(vec![shard0, shard1]);
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize, 1usize]).unwrap();

            super::super::update_shards_after_transaction::<
                MAX_USER_UTXOS,
                MAX_SHARDS_PER_PROGRAM,
                MAX_SEL,
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
            >(&mut builder, &shard_set, &program_script, &fee_rate())
            .unwrap();

            // old rune utxo removed, at least one shard has rune utxo
            let has_rune = shard_set
                .handle_by_index(0)
                .with_ref(|s| s.rune_utxo().is_some())
                .unwrap()
                || shard_set
                    .handle_by_index(1)
                    .with_ref(|s| s.rune_utxo().is_some())
                    .unwrap();
            assert!(has_rune);
        }

        #[test]
        fn propagates_overflow_error_when_all_shards_full() {
            const MAX_USER_UTXOS: usize = 4;
            const MAX_SHARDS_PER_PROGRAM: usize = 4;
            const MAX_SEL: usize = 2;

            let mut builder: satellite_bitcoin::TransactionBuilder<
                MAX_USER_UTXOS,
                MAX_SHARDS_PER_PROGRAM,
                SingleRuneSet,
            > = new_tb!(MAX_USER_UTXOS, MAX_SHARDS_PER_PROGRAM);

            builder.transaction = Transaction {
                version: Version::TWO,
                lock_time: LockTime::ZERO,
                input: vec![],
                output: vec![TxOut {
                    value: Amount::from_sat(1),
                    script_pubkey: ScriptBuf::new(),
                }],
            };

            // Fill both shards to capacity
            let mut shard0 = MockShardZc::default();
            let mut shard1 = MockShardZc::default();
            for i in 0..MockShardZc::btc_utxos_max_len(&shard0) {
                shard0.add_btc_utxo(create_utxo(1, 220, i as u32));
                shard1.add_btc_utxo(create_utxo(1, 221, i as u32));
            }

            let loaders = leak_loaders_from_vec(vec![shard0, shard1]);
            let unselected: ShardSet<MockShardZc, MAX_SEL> = ShardSet::from_loaders(loaders);
            let shard_set = unselected.select_with([0usize, 1usize]).unwrap();

            let err = super::super::update_shards_after_transaction::<
                MAX_USER_UTXOS,
                MAX_SHARDS_PER_PROGRAM,
                MAX_SEL,
                SingleRuneSet,
                satellite_bitcoin::utxo_info::UtxoInfo<SingleRuneSet>,
                MockShardZc,
            >(&mut builder, &shard_set, &ScriptBuf::new(), &fee_rate())
            .unwrap_err();

            assert_eq!(err, ErrorCode::ShardsAreFullOfBtcUtxos.into());
        }
    }
}
