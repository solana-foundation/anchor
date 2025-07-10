#[cfg(feature = "utxo-consolidation")]
use arch_program::{input_to_sign::InputToSign, pubkey::Pubkey, rune::RuneAmount, MAX_BTC_TX_SIZE};

#[cfg(feature = "utxo-consolidation")]
use bitcoin::{Transaction, TxIn, TxOut};

#[cfg(feature = "utxo-consolidation")]
use mempool_oracle_sdk::MempoolInfo;
#[cfg(feature = "utxo-consolidation")]
use saturn_collections::generic::fixed_set::FixedCapacitySet;
use saturn_collections::generic::push_pop::PushPopCollection;

#[cfg(feature = "utxo-consolidation")]
use crate::{
    calc_fee::estimate_tx_size_with_additional_inputs_outputs, error::BitcoinTxError,
    fee_rate::FeeRate, NewPotentialInputsAndOutputs, UtxoInfo,
};

#[cfg(feature = "utxo-consolidation")]
pub fn add_consolidation_utxos<RS, T, C>(
    transaction: &mut Transaction,
    _tx_statuses: &mut MempoolInfo,
    inputs_to_sign: &mut C,
    pool_pubkey: &Pubkey,
    pool_shard_btc_utxos: &[T],
    mempool_fee_rate: &FeeRate,
    new_potential_inputs_and_outputs: &NewPotentialInputsAndOutputs,
    program_input_size: usize,
) -> (u64, usize)
where
    RS: FixedCapacitySet<Item = RuneAmount> + Default,
    T: AsRef<UtxoInfo<RS>>,
    C: PushPopCollection<InputToSign>,
{
    use bitcoin::{OutPoint, ScriptBuf, Sequence, TxIn, Witness};

    let mut total_input_amount = 0;
    let mut additional_inputs_to_consolidate: u32 = 0;

    // Pre-create TxIn template to avoid repeated allocations
    let tx_in_template = TxIn {
        previous_output: OutPoint::default(), // Will be overwritten
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    };

    for utxo_ref in pool_shard_btc_utxos.iter() {
        let utxo = utxo_ref.as_ref();

        // Check consolidation criteria first (cheaper check)
        let should_consolidate = utxo
            .needs_consolidation
            .get()
            .map(|fee_rate| fee_rate >= mempool_fee_rate.0)
            .unwrap_or(false);

        if !should_consolidate {
            continue;
        }

        let outpoint = utxo.meta.to_outpoint();

        // Only include if not already in transaction
        if transaction
            .input
            .iter()
            .any(|input| input.previous_output == outpoint)
        {
            continue;
        }

        // Create TxIn more efficiently by modifying template
        let mut tx_in = tx_in_template.clone();
        tx_in.previous_output = outpoint;

        // If adding an input fails because tx gets too big, stop
        if let Ok(()) = safe_add_input_to_transaction(
            transaction,
            inputs_to_sign,
            pool_pubkey,
            &tx_in,
            &new_potential_inputs_and_outputs,
        ) {
            total_input_amount += utxo.value; // Add btc utxo from pool
            additional_inputs_to_consolidate += 1;
            // All program outputs are confirmed by default.
            // tx_statuses.total_fee += 0;
            // tx_statuses.total_size += 0;
        } else {
            break; // Stop iteration on first error
        }
    }

    let extra_tx_size = calculate_extra_tx_size_for_consolidation(
        program_input_size,
        additional_inputs_to_consolidate,
    );

    (total_input_amount, extra_tx_size)
}

#[cfg(feature = "utxo-consolidation")]
fn calculate_extra_tx_size_for_consolidation(
    program_input_size: usize,
    additional_inputs_to_consolidate: u32,
) -> usize {
    program_input_size * additional_inputs_to_consolidate as usize
}

#[cfg(feature = "utxo-consolidation")]
pub fn safe_add_input_to_transaction<C: PushPopCollection<InputToSign>>(
    transaction: &mut Transaction,
    inputs_to_sign: &mut C,
    signer: &Pubkey,
    input: &TxIn,
    new_potential_inputs_and_outputs: &NewPotentialInputsAndOutputs,
) -> Result<(), BitcoinTxError> {
    let input_index = transaction.input.len() as u32;

    inputs_to_sign
        .push(InputToSign {
            index: input_index,
            signer: *signer,
        })
        .map_err(|_| BitcoinTxError::InputToSignListFull)?;

    transaction.input.push(input.clone());

    let total_size = estimate_tx_size_with_additional_inputs_outputs(
        transaction,
        inputs_to_sign,
        new_potential_inputs_and_outputs,
    )
    .map_err(|_| BitcoinTxError::InputToSignListFull)?;

    if total_size > MAX_BTC_TX_SIZE {
        inputs_to_sign.pop();
        transaction.input.pop();
        return Err(BitcoinTxError::TransactionTooLarge);
    }

    Ok(())
}

#[cfg(feature = "utxo-consolidation")]
pub fn safe_add_output_to_transaction(
    transaction: &mut Transaction,
    inputs_to_sign: &mut Vec<InputToSign>,
    output: &TxOut,
    new_potential_inputs_and_outputs: &NewPotentialInputsAndOutputs,
) -> Result<(), BitcoinTxError> {
    transaction.output.push(output.clone());

    let total_size = estimate_tx_size_with_additional_inputs_outputs(
        transaction,
        inputs_to_sign,
        new_potential_inputs_and_outputs,
    )
    .map_err(|_| BitcoinTxError::InputToSignListFull)?;

    if total_size > MAX_BTC_TX_SIZE {
        transaction.output.pop();
        return Err(BitcoinTxError::TransactionTooLarge);
    }

    Ok(())
}

#[cfg(test)]
#[cfg(feature = "utxo-consolidation")]
mod tests {
    use crate::{
        input_calc::ARCH_INPUT_SIZE,
        utxo_info::{SingleRuneSet, UtxoInfoTrait},
        NewPotentialInputAmount, NewPotentialOutputAmount,
    };

    #[cfg(feature = "utxo-consolidation")]
    use crate::utxo_info::FixedOptionF64;

    use super::*;
    use arch_program::utxo::UtxoMeta;
    use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
    use saturn_collections::generic::push_pop::PushPopError;
    use std::str::FromStr;

    // Helper functions for creating test data
    fn create_mock_transaction() -> Transaction {
        Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
            input: vec![],
            output: vec![],
        }
    }

    fn create_mock_tx_in(outpoint: OutPoint) -> TxIn {
        TxIn {
            previous_output: outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }
    }

    fn create_mock_tx_out(value: u64) -> TxOut {
        TxOut {
            value: Amount::from_sat(value),
            script_pubkey: ScriptBuf::new(),
        }
    }

    fn create_mock_outpoint(txid: [u8; 32], vout: u32) -> OutPoint {
        OutPoint {
            txid: bitcoin::Txid::from_str(&hex::encode(txid)).unwrap(),
            vout,
        }
    }

    fn create_mock_utxo_info(
        txid: [u8; 32],
        vout: u32,
        value: u64,
        needs_consolidation: Option<f64>,
    ) -> UtxoInfo<SingleRuneSet> {
        let mut utxo = UtxoInfo::new(UtxoMeta::from(txid, vout), value);
        if let Some(fee_rate) = needs_consolidation {
            *utxo.needs_consolidation_mut() = FixedOptionF64::some(fee_rate);
        }

        utxo
    }

    #[derive(Debug, Default)]
    struct MockPushPopCollection {
        items: Vec<InputToSign>,
    }

    impl PushPopCollection<InputToSign> for MockPushPopCollection {
        fn push(&mut self, item: InputToSign) -> Result<(), PushPopError> {
            self.items.push(item);
            Ok(())
        }

        fn pop(&mut self) -> Option<InputToSign> {
            self.items.pop()
        }

        fn as_slice(&self) -> &[InputToSign] {
            &self.items
        }

        fn len(&self) -> usize {
            self.items.len()
        }
    }

    #[test]
    fn test_safe_add_input_to_transaction_success() {
        let mut transaction = create_mock_transaction();
        let mut inputs_to_sign = MockPushPopCollection::default();
        let signer = Pubkey::default();
        let input = create_mock_tx_in(create_mock_outpoint([1; 32], 0));

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: None,
            outputs: vec![],
        };

        let result = safe_add_input_to_transaction(
            &mut transaction,
            &mut inputs_to_sign,
            &signer,
            &input,
            &new_potential_inputs_and_outputs,
        );

        assert!(result.is_ok());
        assert_eq!(transaction.input.len(), 1);
        assert_eq!(inputs_to_sign.items.len(), 1);
        assert_eq!(inputs_to_sign.items[0].index, 0);
        assert_eq!(inputs_to_sign.items[0].signer, signer);
    }

    #[test]
    fn test_safe_add_input_to_transaction_with_potential_inputs() {
        let mut transaction = create_mock_transaction();
        let mut inputs_to_sign = MockPushPopCollection::default();
        let signer = Pubkey::default();
        let input = create_mock_tx_in(create_mock_outpoint([1; 32], 0));

        let potential_input = create_mock_tx_in(create_mock_outpoint([2; 32], 0));
        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: Some(NewPotentialInputAmount {
                count: 2,
                item: potential_input,
                signer: Some(signer),
            }),
            outputs: vec![],
        };

        let result = safe_add_input_to_transaction(
            &mut transaction,
            &mut inputs_to_sign,
            &signer,
            &input,
            &new_potential_inputs_and_outputs,
        );

        assert!(result.is_ok());
        // Should only have the original input after rollback
        assert_eq!(transaction.input.len(), 1);
        assert_eq!(inputs_to_sign.items.len(), 1);
    }

    #[test]
    fn test_safe_add_output_to_transaction_success() {
        let mut transaction = create_mock_transaction();
        let mut inputs_to_sign = vec![];
        let output = create_mock_tx_out(1000);

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: None,
            outputs: vec![],
        };

        let result = safe_add_output_to_transaction(
            &mut transaction,
            &mut inputs_to_sign,
            &output,
            &new_potential_inputs_and_outputs,
        );

        assert!(result.is_ok());
        assert_eq!(transaction.output.len(), 1);
        assert_eq!(transaction.output[0].value.to_sat(), 1000);
    }

    #[test]
    fn test_safe_add_output_to_transaction_with_potential_outputs() {
        let mut transaction = create_mock_transaction();
        let mut inputs_to_sign = vec![];
        let output = create_mock_tx_out(1000);

        let potential_output = create_mock_tx_out(500);
        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: None,
            outputs: vec![NewPotentialOutputAmount {
                count: 2,
                item: potential_output,
            }],
        };

        let result = safe_add_output_to_transaction(
            &mut transaction,
            &mut inputs_to_sign,
            &output,
            &new_potential_inputs_and_outputs,
        );

        assert!(result.is_ok());
        // Should only have the original output after rollback
        assert_eq!(transaction.output.len(), 1);
        assert_eq!(transaction.output[0].value.to_sat(), 1000);
    }

    #[test]
    fn test_add_consolidation_utxos_success() {
        let mut transaction = create_mock_transaction();
        let mut tx_statuses = MempoolInfo::default();
        let mut inputs_to_sign = MockPushPopCollection::default();
        let pool_pubkey = Pubkey::default();
        let mempool_fee_rate = FeeRate::try_from(10.0).unwrap();

        let utxos = vec![
            create_mock_utxo_info([1; 32], 0, 10000, Some(15.0)), // Should be consolidated (15 >= 10)
            create_mock_utxo_info([2; 32], 0, 20000, Some(5.0)), // Should not be consolidated (5 < 10)
            create_mock_utxo_info([3; 32], 0, 30000, Some(20.0)), // Should be consolidated (20 >= 10)
        ];

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: None,
            outputs: vec![],
        };

        let (total_amount, extra_size) = add_consolidation_utxos(
            &mut transaction,
            &mut tx_statuses,
            &mut inputs_to_sign,
            &pool_pubkey,
            &utxos,
            &mempool_fee_rate,
            &new_potential_inputs_and_outputs,
            ARCH_INPUT_SIZE,
        );

        assert_eq!(total_amount, 40000); // 10000 + 30000
        assert_eq!(transaction.input.len(), 2);
        assert_eq!(inputs_to_sign.items.len(), 2);
        assert!(extra_size > 0);
    }

    #[test]
    fn test_add_consolidation_utxos_no_consolidation_needed() {
        let mut transaction = create_mock_transaction();
        let mut tx_statuses = MempoolInfo::default();
        let mut inputs_to_sign = MockPushPopCollection::default();
        let pool_pubkey = Pubkey::default();
        let mempool_fee_rate = FeeRate::try_from(10.0).unwrap();

        let utxos = vec![
            create_mock_utxo_info([1; 32], 0, 10000, Some(5.0)), // Should not be consolidated
            create_mock_utxo_info([2; 32], 0, 20000, None),      // No consolidation info
        ];

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: None,
            outputs: vec![],
        };

        let (total_amount, extra_size) = add_consolidation_utxos(
            &mut transaction,
            &mut tx_statuses,
            &mut inputs_to_sign,
            &pool_pubkey,
            &utxos,
            &mempool_fee_rate,
            &new_potential_inputs_and_outputs,
            ARCH_INPUT_SIZE,
        );

        assert_eq!(total_amount, 0);
        assert_eq!(transaction.input.len(), 0);
        assert_eq!(inputs_to_sign.items.len(), 0);
        assert_eq!(extra_size, 0);
    }

    #[test]
    fn test_add_consolidation_utxos_skip_existing_inputs() {
        let mut transaction = create_mock_transaction();
        let outpoint = create_mock_outpoint([1; 32], 0);

        // Add an input that matches one of the UTXOs
        transaction.input.push(create_mock_tx_in(outpoint.clone()));

        let mut tx_statuses = MempoolInfo::default();
        let mut inputs_to_sign = MockPushPopCollection::default();
        let pool_pubkey = Pubkey::default();
        let mempool_fee_rate = FeeRate::try_from(10.0).unwrap();

        let utxos = vec![
            create_mock_utxo_info([1; 32], 0, 10000, Some(15.0)), // Same as existing input
            create_mock_utxo_info([2; 32], 0, 20000, Some(15.0)), // Different, should be added
        ];

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: None,
            outputs: vec![],
        };

        let (total_amount, _) = add_consolidation_utxos(
            &mut transaction,
            &mut tx_statuses,
            &mut inputs_to_sign,
            &pool_pubkey,
            &utxos,
            &mempool_fee_rate,
            &new_potential_inputs_and_outputs,
            ARCH_INPUT_SIZE,
        );

        // Should only add the second UTXO (first one is already in transaction)
        assert_eq!(total_amount, 20000);
        assert_eq!(transaction.input.len(), 2); // 1 existing + 1 new
        assert_eq!(inputs_to_sign.items.len(), 1); // Only 1 new signer
    }

    #[test]
    fn test_calculate_extra_tx_size_for_consolidation() {
        let result = calculate_extra_tx_size_for_consolidation(ARCH_INPUT_SIZE, 0);
        assert_eq!(result, 0);

        let result = calculate_extra_tx_size_for_consolidation(ARCH_INPUT_SIZE, 1);
        assert_eq!(result, ARCH_INPUT_SIZE);

        let result = calculate_extra_tx_size_for_consolidation(ARCH_INPUT_SIZE, 5);
        assert_eq!(result, ARCH_INPUT_SIZE * 5);
    }

    #[test]
    fn test_new_potential_inputs_and_outputs_struct() {
        let input_item = create_mock_tx_in(create_mock_outpoint([1; 32], 0));
        let output_item = create_mock_tx_out(1000);
        let signer = Pubkey::default();

        let new_potential = NewPotentialInputsAndOutputs {
            inputs: Some(NewPotentialInputAmount {
                count: 2,
                item: input_item.clone(),
                signer: Some(signer),
            }),
            outputs: vec![
                NewPotentialOutputAmount {
                    count: 3,
                    item: output_item.clone(),
                },
                NewPotentialOutputAmount {
                    count: 1,
                    item: create_mock_tx_out(2000),
                },
            ],
        };

        assert!(new_potential.inputs.is_some());
        assert_eq!(new_potential.outputs.len(), 2);

        if let Some(ref inputs) = new_potential.inputs {
            assert_eq!(inputs.count, 2);
            assert_eq!(inputs.item.previous_output, input_item.previous_output);
            assert_eq!(inputs.signer, Some(signer));
        }

        assert_eq!(new_potential.outputs[0].count, 3);
        assert_eq!(new_potential.outputs[0].item.value, output_item.value);
        assert_eq!(new_potential.outputs[1].count, 1);
        assert_eq!(new_potential.outputs[1].item.value.to_sat(), 2000);
    }

    #[test]
    fn test_safe_add_input_transaction_too_large() {
        let mut transaction = create_mock_transaction();

        // Fill transaction with many outputs to make it close to MAX_BTC_TX_SIZE
        // Each output has ~41 bytes (8 bytes value + ~33 bytes script), so add many
        for i in 0..2000 {
            transaction.output.push(create_mock_tx_out(1000 + i));
        }

        let mut inputs_to_sign = MockPushPopCollection::default();
        let signer = Pubkey::default();
        let input = create_mock_tx_in(create_mock_outpoint([1; 32], 0));

        // Add potential inputs that would make the transaction too large
        let potential_input = create_mock_tx_in(create_mock_outpoint([2; 32], 0));
        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: Some(NewPotentialInputAmount {
                count: 1000, // Many potential inputs to exceed size limit
                item: potential_input,
                signer: Some(signer),
            }),
            outputs: vec![],
        };

        let result = safe_add_input_to_transaction(
            &mut transaction,
            &mut inputs_to_sign,
            &signer,
            &input,
            &new_potential_inputs_and_outputs,
        );

        // Should fail due to transaction being too large
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), BitcoinTxError::TransactionTooLarge);

        // Transaction should be unchanged after failed addition
        assert_eq!(transaction.input.len(), 0);
        assert_eq!(inputs_to_sign.items.len(), 0);
    }

    #[test]
    fn test_safe_add_output_transaction_too_large() {
        let mut transaction = create_mock_transaction();

        // Fill transaction with many outputs to make it close to MAX_BTC_TX_SIZE
        for i in 0..2000 {
            transaction.output.push(create_mock_tx_out(1000 + i));
        }

        let mut inputs_to_sign = vec![];
        let output = create_mock_tx_out(5000);

        // Add potential outputs that would make the transaction too large
        let potential_output = create_mock_tx_out(500);
        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: None,
            outputs: vec![NewPotentialOutputAmount {
                count: 1000, // Many potential outputs to exceed size limit
                item: potential_output,
            }],
        };

        let result = safe_add_output_to_transaction(
            &mut transaction,
            &mut inputs_to_sign,
            &output,
            &new_potential_inputs_and_outputs,
        );

        // Should fail due to transaction being too large
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), BitcoinTxError::TransactionTooLarge);

        // Transaction should be unchanged after failed addition (back to original 2000 outputs)
        assert_eq!(transaction.output.len(), 2000);
    }

    #[test]
    fn test_add_consolidation_utxos_empty_collection() {
        let mut transaction = create_mock_transaction();
        let mut tx_statuses = MempoolInfo::default();
        let mut inputs_to_sign = MockPushPopCollection::default();
        let pool_pubkey = Pubkey::default();
        let mempool_fee_rate = FeeRate::try_from(10.0).unwrap();

        let utxos: Vec<UtxoInfo<SingleRuneSet>> = vec![]; // Empty collection

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: None,
            outputs: vec![],
        };

        let (total_amount, extra_size) = add_consolidation_utxos(
            &mut transaction,
            &mut tx_statuses,
            &mut inputs_to_sign,
            &pool_pubkey,
            &utxos,
            &mempool_fee_rate,
            &new_potential_inputs_and_outputs,
            ARCH_INPUT_SIZE,
        );

        assert_eq!(total_amount, 0);
        assert_eq!(extra_size, 0);
        assert_eq!(transaction.input.len(), 0);
        assert_eq!(inputs_to_sign.items.len(), 0);
    }

    #[test]
    fn test_add_consolidation_utxos_mixed_scenarios() {
        let mut transaction = create_mock_transaction();
        let mut tx_statuses = MempoolInfo::default();
        let mut inputs_to_sign = MockPushPopCollection::default();
        let pool_pubkey = Pubkey::default();
        let mempool_fee_rate = FeeRate::try_from(10.0).unwrap();

        let utxos = vec![
            create_mock_utxo_info([1; 32], 0, 10000, Some(15.0)), // Should consolidate
            create_mock_utxo_info([2; 32], 0, 20000, None),       // No consolidation info
            create_mock_utxo_info([3; 32], 0, 30000, Some(5.0)),  // Below threshold
            create_mock_utxo_info([4; 32], 0, 40000, Some(10.0)), // Exactly at threshold
            create_mock_utxo_info([5; 32], 0, 50000, Some(25.0)), // Well above threshold
        ];

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: None,
            outputs: vec![],
        };

        let (total_amount, extra_size) = add_consolidation_utxos(
            &mut transaction,
            &mut tx_statuses,
            &mut inputs_to_sign,
            &pool_pubkey,
            &utxos,
            &mempool_fee_rate,
            &new_potential_inputs_and_outputs,
            ARCH_INPUT_SIZE,
        );

        // Should consolidate: [1] (15.0 >= 10.0), [4] (10.0 >= 10.0), [5] (25.0 >= 10.0)
        // Should NOT consolidate: [2] (None), [3] (5.0 < 10.0)
        assert_eq!(total_amount, 100000); // 10000 + 40000 + 50000
        assert_eq!(transaction.input.len(), 3);
        assert_eq!(inputs_to_sign.items.len(), 3);
        assert!(extra_size > 0);
    }

    #[test]
    fn test_integration_complex_scenario() {
        let mut transaction = create_mock_transaction();
        let mut inputs_to_sign = MockPushPopCollection::default();
        let signer = Pubkey::default();

        // Step 1: Add an initial input
        let input1 = create_mock_tx_in(create_mock_outpoint([1; 32], 0));
        let result = safe_add_input_to_transaction(
            &mut transaction,
            &mut inputs_to_sign,
            &signer,
            &input1,
            &NewPotentialInputsAndOutputs {
                inputs: None,
                outputs: vec![],
            },
        );
        assert!(result.is_ok());

        // Step 2: Add an output with potential outputs
        let output1 = create_mock_tx_out(50000);
        let potential_output = create_mock_tx_out(1000);
        let result = safe_add_output_to_transaction(
            &mut transaction,
            &mut inputs_to_sign.items,
            &output1,
            &NewPotentialInputsAndOutputs {
                inputs: None,
                outputs: vec![NewPotentialOutputAmount {
                    count: 2,
                    item: potential_output,
                }],
            },
        );
        assert!(result.is_ok());

        // Step 3: Add consolidation UTXOs
        let mut tx_statuses = MempoolInfo::default();
        let mempool_fee_rate = FeeRate::try_from(10.0).unwrap();
        let utxos = vec![
            create_mock_utxo_info([2; 32], 0, 25000, Some(15.0)),
            create_mock_utxo_info([3; 32], 0, 35000, Some(20.0)),
        ];

        let (consolidation_amount, _) = add_consolidation_utxos(
            &mut transaction,
            &mut tx_statuses,
            &mut inputs_to_sign,
            &signer,
            &utxos,
            &mempool_fee_rate,
            &NewPotentialInputsAndOutputs {
                inputs: None,
                outputs: vec![],
            },
            ARCH_INPUT_SIZE,
        );

        // Final state verification
        assert_eq!(transaction.input.len(), 3); // 1 initial + 2 consolidation
        assert_eq!(transaction.output.len(), 1); // 1 output (potential ones rolled back)
        assert_eq!(inputs_to_sign.items.len(), 3); // All inputs have signers
        assert_eq!(consolidation_amount, 60000); // 25000 + 35000

        // Verify all signers have correct indices
        for (i, input_to_sign) in inputs_to_sign.items.iter().enumerate() {
            assert_eq!(input_to_sign.index, i as u32);
            assert_eq!(input_to_sign.signer, signer);
        }
    }
}
