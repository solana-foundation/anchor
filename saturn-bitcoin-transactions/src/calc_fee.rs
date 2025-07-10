use std::cmp::min;

use arch_program::input_to_sign::InputToSign;
use bitcoin::{Amount, ScriptBuf, Transaction, TxOut};
use mempool_oracle_sdk::MempoolInfo;
use saturn_collections::generic::push_pop::{PushPopCollection, PushPopError};
use saturn_safe_math::{safe_add, safe_sub};

use crate::{
    constants::DUST_LIMIT,
    error::BitcoinTxError,
    fee_rate::FeeRate,
    input_calc::{WITNESS_WEIGHT_BYTES, WITNESS_WEIGHT_OVERHEAD},
    NewPotentialInputAmount, NewPotentialInputsAndOutputs, NewPotentialOutputAmount,
};

pub(crate) fn estimate_final_tx_total_size(
    transaction: &Transaction,
    inputs_to_sign: &[InputToSign],
) -> usize {
    let size = transaction.total_size();

    size + inputs_to_sign.len() * WITNESS_WEIGHT_BYTES + WITNESS_WEIGHT_OVERHEAD
}

pub(crate) fn estimate_final_tx_vsize(
    transaction: &Transaction,
    inputs_to_sign: &[InputToSign],
) -> usize {
    let vsize = transaction.vsize();

    vsize + (inputs_to_sign.len() * WITNESS_WEIGHT_BYTES + WITNESS_WEIGHT_OVERHEAD) / 4
}

pub(crate) fn calculate_fees_for_transaction(
    _remaining_btc: u64,
    transaction: &mut Transaction,
    inputs_to_sign: &[InputToSign],
    total_size_of_pending_utxos: usize,
    fee_rate: &FeeRate,
) -> Result<(u64, u64), BitcoinTxError> {
    let base_tx_size = estimate_final_tx_vsize(transaction, inputs_to_sign);
    let total_size = safe_add(base_tx_size, total_size_of_pending_utxos as usize)?;

    let base_fee = fee_rate.fee(base_tx_size);
    let total_fee = fee_rate.fee(total_size);

    Ok((total_fee.to_sat(), base_fee.to_sat()))
}

pub(crate) fn adjust_transaction_to_pay_fees(
    transaction: &mut Transaction,
    inputs_to_sign: &[InputToSign],
    tx_statuses: &MempoolInfo,
    total_btc_amount: u64,
    address_to_send_remaining_btc: Option<ScriptBuf>,
    fee_rate: &FeeRate,
) -> Result<(), BitcoinTxError> {
    let total_btc_used = transaction
        .output
        .iter()
        .map(|output| output.value.to_sat())
        .sum::<u64>();

    let (total_size_of_pending_utxos, total_fee_paid_of_pending_utxos) =
        (tx_statuses.total_size as usize, tx_statuses.total_fee);

    // Calculate remaining BTC after outputs
    let remaining_btc = safe_sub(total_btc_amount, total_btc_used)
        .map_err(|_| BitcoinTxError::NotEnoughAmountToCoverFees)?;

    // Get change without ancestors
    let (total_fee_with_ancestors, total_fee_without_ancestors) = calculate_fees_for_transaction(
        remaining_btc,
        transaction,
        inputs_to_sign,
        total_size_of_pending_utxos,
        fee_rate,
    )?;

    // Get available change with and without ancestors
    let available_for_change_without_ancestors =
        safe_sub(remaining_btc, total_fee_without_ancestors)
            .map_err(|_| BitcoinTxError::NotEnoughAmountToCoverFees)?;

    let available_for_change_with_ancestors = safe_sub(
        safe_add(remaining_btc, total_fee_paid_of_pending_utxos)?,
        total_fee_with_ancestors,
    )
    .map_err(|_| BitcoinTxError::NotEnoughAmountToCoverFees)?;

    // Get the minimum. We want to cover both the ancestors fees and ours.
    // But we don't want to use ancestors fees to pay ours.
    let available_for_change = min(
        available_for_change_without_ancestors,
        available_for_change_with_ancestors,
    );

    // Only add change output if we have enough to cover dust limit
    if let Some(change_script) = address_to_send_remaining_btc {
        if available_for_change >= DUST_LIMIT {
            // Add change output
            transaction.output.push(TxOut {
                value: Amount::from_sat(available_for_change),
                script_pubkey: change_script,
            });

            // Recalculate fees with change output
            let (_, new_total_fee_without_ancestors) = calculate_fees_for_transaction(
                remaining_btc,
                transaction,
                inputs_to_sign,
                total_size_of_pending_utxos,
                fee_rate,
            )?;

            let fee_difference =
                safe_sub(new_total_fee_without_ancestors, total_fee_without_ancestors)?;

            match safe_sub(available_for_change, fee_difference) {
                Ok(new_remaining_btc) if new_remaining_btc >= DUST_LIMIT => {
                    // Update change output with final amount
                    transaction.output.last_mut().unwrap().value =
                        Amount::from_sat(new_remaining_btc);
                }
                _ => {
                    // If we can't afford the change output or it would be dust, simply remove it
                    transaction.output.pop();
                }
            }
        }
    }

    Ok(())
}

pub fn estimate_tx_size_with_additional_inputs_outputs<C: PushPopCollection<InputToSign>>(
    transaction: &mut Transaction,
    inputs_to_sign: &mut C,
    new_potential_inputs_and_outputs: &NewPotentialInputsAndOutputs,
) -> Result<usize, BitcoinTxError> {
    add_reserved_inputs_and_outputs(
        transaction,
        inputs_to_sign,
        new_potential_inputs_and_outputs,
    )
    .map_err(|_| BitcoinTxError::InputToSignListFull)?;

    let total_size = estimate_final_tx_total_size(transaction, inputs_to_sign.as_slice());

    rollback_potential_inputs_and_outputs(
        transaction,
        inputs_to_sign,
        new_potential_inputs_and_outputs,
    );

    Ok(total_size)
}

pub fn estimate_tx_vsize_with_additional_inputs_outputs<C: PushPopCollection<InputToSign>>(
    transaction: &mut Transaction,
    inputs_to_sign: &mut C,
    new_potential_inputs_and_outputs: &NewPotentialInputsAndOutputs,
) -> Result<usize, BitcoinTxError> {
    add_reserved_inputs_and_outputs(
        transaction,
        inputs_to_sign,
        new_potential_inputs_and_outputs,
    )
    .map_err(|_| BitcoinTxError::InputToSignListFull)?;

    let total_vsize = estimate_final_tx_vsize(transaction, inputs_to_sign.as_slice());

    rollback_potential_inputs_and_outputs(
        transaction,
        inputs_to_sign,
        new_potential_inputs_and_outputs,
    );

    Ok(total_vsize)
}

fn add_reserved_inputs_and_outputs<C: PushPopCollection<InputToSign>>(
    transaction: &mut Transaction,
    inputs_to_sign: &mut C,
    new_potential_inputs_and_outputs: &NewPotentialInputsAndOutputs,
) -> Result<(), PushPopError> {
    if let Some(NewPotentialInputAmount {
        count,
        ref item,
        signer,
    }) = new_potential_inputs_and_outputs.inputs
    {
        // Pre-allocate capacity to avoid repeated reallocations
        transaction.input.reserve(count);

        let initial_input_len = transaction.input.len();
        for i in 0..count {
            if let Some(signer) = signer {
                inputs_to_sign.push(InputToSign {
                    index: (initial_input_len + i) as u32,
                    signer,
                })?;
            }

            transaction.input.push(item.clone());
        }
    }

    for NewPotentialOutputAmount { count, item } in new_potential_inputs_and_outputs.outputs.iter()
    {
        for _ in 0..*count {
            transaction.output.push(item.clone());
        }
    }

    Ok(())
}

fn rollback_potential_inputs_and_outputs<C: PushPopCollection<InputToSign>>(
    transaction: &mut Transaction,
    inputs_to_sign: &mut C,
    new_potential_inputs_and_outputs: &NewPotentialInputsAndOutputs,
) {
    if let Some(NewPotentialInputAmount {
        count,
        item: _,
        signer,
    }) = new_potential_inputs_and_outputs.inputs
    {
        let new_input_len = transaction.input.len() - count;
        transaction.input.truncate(new_input_len);

        // Also rollback inputs_to_sign if signer was provided
        if signer.is_some() {
            for _ in 0..count {
                inputs_to_sign.pop();
            }
        }
    }

    for NewPotentialOutputAmount { count, item: _ } in
        new_potential_inputs_and_outputs.outputs.iter()
    {
        let new_output_len = transaction.output.len() - *count;
        transaction.output.truncate(new_output_len);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        input_calc::{CONTROL_BLOCK_SIZE, REDEEM_SCRIPT_SIZE},
        NewPotentialInputAmount, NewPotentialOutputAmount,
    };

    use super::*;
    use arch_program::pubkey::Pubkey;
    use bitcoin::{
        absolute::LockTime, key::constants::SCHNORR_SIGNATURE_SIZE, transaction::Version, Address,
        Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
    };
    use saturn_collections::generic::push_pop::PushPopError;
    use std::str::FromStr;

    enum AddressType {
        Program,
        Account,
        User,
    }

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

    fn create_mock_input_to_sign(index: u32) -> InputToSign {
        InputToSign {
            index,
            signer: Pubkey::default(),
        }
    }

    fn create_mock_outpoint(txid: [u8; 32], vout: u32) -> OutPoint {
        OutPoint {
            txid: bitcoin::Txid::from_str(&hex::encode(txid)).unwrap(),
            vout,
        }
    }

    fn add_fake_witness_to_transaction(
        transaction: &mut Transaction,
        inputs_to_sign: &[InputToSign],
    ) {
        inputs_to_sign.iter().for_each(|input| {
            // Construct the witness with stack-allocated arrays
            let signature = [0u8; SCHNORR_SIGNATURE_SIZE];
            let redeem_script = [0u8; REDEEM_SCRIPT_SIZE];
            let control_block = [0u8; CONTROL_BLOCK_SIZE];

            let witness_items: [&[u8]; 3] = [&signature, &redeem_script, &control_block];

            transaction.input[input.index as usize].witness = Witness::from_slice(&witness_items);
        });
    }

    fn create_mock_address(address_type: AddressType) -> Address {
        match address_type {
            AddressType::Program => Address::from_str(
                &String::from_str("bcrt1qhwz8mxa0e5l7wep79rk765swffkmqvxzdmz5lt").unwrap(),
            )
            .unwrap()
            .require_network(bitcoin::Network::Regtest)
            .unwrap(),
            AddressType::Account => Address::from_str(
                &String::from_str(
                    "bcrt1ptccryszg6u3xvppg3scnh9xac5ke7qypvtsxqzgzut9j92k3h5tqfdxs47",
                )
                .unwrap(),
            )
            .unwrap()
            .require_network(bitcoin::Network::Regtest)
            .unwrap(),
            AddressType::User => Address::from_str(
                &String::from_str(
                    "bcrt1pkv5vcfxj4lj3anfnj7s8758ysw5kw54ukk0jdvs80gw6yg2c4v3s24lt2u",
                )
                .unwrap(),
            )
            .unwrap()
            .require_network(bitcoin::Network::Regtest)
            .unwrap(),
        }
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
    fn test_add_reserved_inputs_and_outputs() {
        let mut transaction = create_mock_transaction();
        let mut inputs_to_sign = MockPushPopCollection::default();
        let signer = Pubkey::default();

        let potential_input = create_mock_tx_in(create_mock_outpoint([1; 32], 0));
        let potential_output = create_mock_tx_out(500);

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: Some(NewPotentialInputAmount {
                count: 2,
                item: potential_input,
                signer: Some(signer),
            }),
            outputs: vec![NewPotentialOutputAmount {
                count: 3,
                item: potential_output,
            }],
        };

        add_reserved_inputs_and_outputs(
            &mut transaction,
            &mut inputs_to_sign,
            &new_potential_inputs_and_outputs,
        );

        assert_eq!(transaction.input.len(), 2);
        assert_eq!(transaction.output.len(), 3);
        assert_eq!(inputs_to_sign.items.len(), 2);

        // Check that input indices are correct when starting from empty transaction
        assert_eq!(inputs_to_sign.items[0].index, 0);
        assert_eq!(inputs_to_sign.items[1].index, 1);
    }

    #[test]
    fn test_add_reserved_inputs_and_outputs_no_signer() {
        let mut transaction = create_mock_transaction();
        let mut inputs_to_sign = MockPushPopCollection::default();

        let potential_input = create_mock_tx_in(create_mock_outpoint([1; 32], 0));

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: Some(NewPotentialInputAmount {
                count: 2,
                item: potential_input,
                signer: None,
            }),
            outputs: vec![],
        };

        add_reserved_inputs_and_outputs(
            &mut transaction,
            &mut inputs_to_sign,
            &new_potential_inputs_and_outputs,
        );

        assert_eq!(transaction.input.len(), 2);
        assert_eq!(inputs_to_sign.items.len(), 0); // No signers should be added
    }

    #[test]
    fn test_rollback_potential_inputs_and_outputs() {
        let mut transaction = create_mock_transaction();

        // Add some initial inputs and outputs
        transaction
            .input
            .push(create_mock_tx_in(create_mock_outpoint([1; 32], 0)));
        transaction
            .input
            .push(create_mock_tx_in(create_mock_outpoint([2; 32], 0)));
        transaction
            .input
            .push(create_mock_tx_in(create_mock_outpoint([3; 32], 0)));

        transaction.output.push(create_mock_tx_out(1000));
        transaction.output.push(create_mock_tx_out(2000));
        transaction.output.push(create_mock_tx_out(3000));
        transaction.output.push(create_mock_tx_out(4000));

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: Some(NewPotentialInputAmount {
                count: 2, // Remove last 2 inputs
                item: create_mock_tx_in(create_mock_outpoint([0; 32], 0)),
                signer: None,
            }),
            outputs: vec![NewPotentialOutputAmount {
                count: 3, // Remove last 3 outputs
                item: create_mock_tx_out(0),
            }],
        };

        let mut mock_inputs_to_sign = MockPushPopCollection::default();
        rollback_potential_inputs_and_outputs(
            &mut transaction,
            &mut mock_inputs_to_sign,
            &new_potential_inputs_and_outputs,
        );

        assert_eq!(transaction.input.len(), 1);
        assert_eq!(transaction.output.len(), 1);
        assert_eq!(transaction.output[0].value.to_sat(), 1000);
    }

    #[test]
    fn test_rollback_with_no_potential_inputs() {
        let mut transaction = create_mock_transaction();
        transaction.output.push(create_mock_tx_out(1000));
        transaction.output.push(create_mock_tx_out(2000));

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: None, // No potential inputs
            outputs: vec![NewPotentialOutputAmount {
                count: 1, // Remove 1 output
                item: create_mock_tx_out(0),
            }],
        };

        let mut mock_inputs_to_sign = MockPushPopCollection::default();
        rollback_potential_inputs_and_outputs(
            &mut transaction,
            &mut mock_inputs_to_sign,
            &new_potential_inputs_and_outputs,
        );

        // Only outputs should be affected
        assert_eq!(transaction.input.len(), 0);
        assert_eq!(transaction.output.len(), 1);
        assert_eq!(transaction.output[0].value.to_sat(), 1000);
    }

    #[test]
    fn test_rollback_with_no_potential_outputs() {
        let mut transaction = create_mock_transaction();
        transaction
            .input
            .push(create_mock_tx_in(create_mock_outpoint([1; 32], 0)));
        transaction
            .input
            .push(create_mock_tx_in(create_mock_outpoint([2; 32], 0)));

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: Some(NewPotentialInputAmount {
                count: 1, // Remove 1 input
                item: create_mock_tx_in(create_mock_outpoint([0; 32], 0)),
                signer: None,
            }),
            outputs: vec![], // No potential outputs
        };

        let mut mock_inputs_to_sign = MockPushPopCollection::default();
        rollback_potential_inputs_and_outputs(
            &mut transaction,
            &mut mock_inputs_to_sign,
            &new_potential_inputs_and_outputs,
        );

        // Only inputs should be affected
        assert_eq!(transaction.input.len(), 1);
        assert_eq!(transaction.output.len(), 0);
    }

    #[test]
    fn test_add_reserved_inputs_with_existing_inputs() {
        let mut transaction = create_mock_transaction();
        let mut inputs_to_sign = MockPushPopCollection::default();
        let signer = Pubkey::default();

        // Add some existing inputs first
        transaction
            .input
            .push(create_mock_tx_in(create_mock_outpoint([1; 32], 0)));
        transaction
            .input
            .push(create_mock_tx_in(create_mock_outpoint([2; 32], 0)));
        inputs_to_sign.push(InputToSign { index: 0, signer });
        inputs_to_sign.push(InputToSign { index: 1, signer });

        let potential_input = create_mock_tx_in(create_mock_outpoint([3; 32], 0));
        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: Some(NewPotentialInputAmount {
                count: 3,
                item: potential_input,
                signer: Some(signer),
            }),
            outputs: vec![],
        };

        add_reserved_inputs_and_outputs(
            &mut transaction,
            &mut inputs_to_sign,
            &new_potential_inputs_and_outputs,
        );

        // Should have 2 existing + 3 new = 5 total inputs
        assert_eq!(transaction.input.len(), 5);
        assert_eq!(inputs_to_sign.items.len(), 5);

        // Check that new input indices are correct (should start from index 2)
        assert_eq!(inputs_to_sign.items[2].index, 2);
        assert_eq!(inputs_to_sign.items[3].index, 3);
        assert_eq!(inputs_to_sign.items[4].index, 4);
    }

    #[test]
    fn test_rollback_with_signers() {
        let mut transaction = create_mock_transaction();
        let mut inputs_to_sign = MockPushPopCollection::default();
        let signer = Pubkey::default();

        // Set up transaction with some inputs and corresponding signers
        for i in 0..5 {
            transaction
                .input
                .push(create_mock_tx_in(create_mock_outpoint([i as u8; 32], 0)));
            inputs_to_sign.push(InputToSign {
                index: i as u32,
                signer,
            });
        }

        // Add some outputs
        for i in 0..4 {
            transaction.output.push(create_mock_tx_out(1000 * (i + 1)));
        }

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: Some(NewPotentialInputAmount {
                count: 2, // Remove last 2 inputs
                item: create_mock_tx_in(create_mock_outpoint([0; 32], 0)),
                signer: Some(signer), // With signer - should also rollback inputs_to_sign
            }),
            outputs: vec![NewPotentialOutputAmount {
                count: 3, // Remove last 3 outputs
                item: create_mock_tx_out(0),
            }],
        };

        rollback_potential_inputs_and_outputs(
            &mut transaction,
            &mut inputs_to_sign,
            &new_potential_inputs_and_outputs,
        );

        // Should have 3 inputs and 1 output left
        assert_eq!(transaction.input.len(), 3);
        assert_eq!(transaction.output.len(), 1);
        assert_eq!(inputs_to_sign.items.len(), 3); // Should also rollback signers

        // Verify remaining signers have correct indices
        for i in 0..3 {
            assert_eq!(inputs_to_sign.items[i].index, i as u32);
        }
    }

    #[test]
    fn test_add_reserved_with_zero_count() {
        let mut transaction = create_mock_transaction();
        let mut inputs_to_sign = MockPushPopCollection::default();

        let potential_input = create_mock_tx_in(create_mock_outpoint([1; 32], 0));
        let potential_output = create_mock_tx_out(500);

        let new_potential_inputs_and_outputs = NewPotentialInputsAndOutputs {
            inputs: Some(NewPotentialInputAmount {
                count: 0, // Zero count
                item: potential_input,
                signer: Some(Pubkey::default()),
            }),
            outputs: vec![NewPotentialOutputAmount {
                count: 0, // Zero count
                item: potential_output,
            }],
        };

        add_reserved_inputs_and_outputs(
            &mut transaction,
            &mut inputs_to_sign,
            &new_potential_inputs_and_outputs,
        );

        // Nothing should be added
        assert_eq!(transaction.input.len(), 0);
        assert_eq!(transaction.output.len(), 0);
        assert_eq!(inputs_to_sign.items.len(), 0);
    }

    #[test]
    fn test_estimate_final_tx_total_and_vsize() {
        use crate::input_calc::{WITNESS_WEIGHT_BYTES, WITNESS_WEIGHT_OVERHEAD};

        let mut transaction = create_mock_transaction();
        transaction
            .input
            .push(create_mock_tx_in(create_mock_outpoint([1; 32], 0)));

        // One signer corresponds to the single input we just added.
        let signer = Pubkey::default();
        let inputs_to_sign_vec = vec![InputToSign { index: 0, signer }];

        // --- Total size ---
        let expected_total_size =
            transaction.total_size() + WITNESS_WEIGHT_BYTES + WITNESS_WEIGHT_OVERHEAD;
        let calculated_total_size =
            super::estimate_final_tx_total_size(&transaction, &inputs_to_sign_vec);
        assert_eq!(calculated_total_size, expected_total_size);

        // --- Virtual size ---
        let expected_vsize =
            transaction.vsize() + (WITNESS_WEIGHT_BYTES + WITNESS_WEIGHT_OVERHEAD) / 4;
        let calculated_vsize = super::estimate_final_tx_vsize(&transaction, &inputs_to_sign_vec);
        assert_eq!(calculated_vsize, expected_vsize);
    }

    #[test]
    fn test_calculate_fees_for_transaction() {
        let mut transaction = create_mock_transaction();
        // Add a dummy output so the transaction isn't empty
        transaction.output.push(create_mock_tx_out(10_000));

        let inputs_to_sign: Vec<InputToSign> = Vec::new();
        let fee_rate = FeeRate::try_from(2.0).unwrap(); // 2 sats/vB

        // Assume the user still has 100k sats available to cover fees/change
        let (total_fee, base_fee) = super::calculate_fees_for_transaction(
            100_000,
            &mut transaction,
            &inputs_to_sign,
            0, // No ancestor transactions
            &fee_rate,
        )
        .expect("fee calculation should succeed");

        // Manual calculation for comparison
        let expected_base_fee = fee_rate
            .fee(super::estimate_final_tx_vsize(
                &transaction,
                &inputs_to_sign,
            ))
            .to_sat();
        assert_eq!(base_fee, expected_base_fee);
        assert_eq!(total_fee, expected_base_fee); // No pending UTXOs so totals match
    }

    #[test]
    fn test_adjust_transaction_to_pay_fees_adds_change_output() {
        use crate::constants::DUST_LIMIT;

        // Build a transaction where 50_000 sats are already assigned to outputs
        let mut transaction = create_mock_transaction();
        transaction.output.push(create_mock_tx_out(50_000));

        let inputs_to_sign: Vec<InputToSign> = Vec::new();
        let tx_statuses = MempoolInfo {
            total_fee: 0,
            total_size: 0,
        };

        // Provide 100_000 sats in total – 50_000 already used, 50_000 remain
        let total_btc_amount = 100_000u64;
        let fee_rate = FeeRate::try_from(1.0).unwrap(); // 1 sat/vB (very small to guarantee change output)

        let change_script = ScriptBuf::new();
        super::adjust_transaction_to_pay_fees(
            &mut transaction,
            &inputs_to_sign,
            &tx_statuses,
            total_btc_amount,
            Some(change_script.clone()),
            &fee_rate,
        )
        .expect("adjust_transaction_to_pay_fees should succeed");

        // We expect a second output containing the change (non-dust)
        assert_eq!(transaction.output.len(), 2);
        let change_output = &transaction.output[1];
        assert!(change_output.value.to_sat() >= DUST_LIMIT);
        assert_eq!(change_output.script_pubkey, change_script);
    }

    #[test]
    fn test_estimate_size_with_additional_inputs_outputs() {
        let mut transaction = create_mock_transaction();
        let mut inputs_to_sign = MockPushPopCollection::default();
        let signer = Pubkey::default();

        let potential_input = create_mock_tx_in(create_mock_outpoint([1; 32], 0));
        let potential_output = create_mock_tx_out(1_000);

        let new_potential = NewPotentialInputsAndOutputs {
            inputs: Some(NewPotentialInputAmount {
                count: 2,
                item: potential_input,
                signer: Some(signer),
            }),
            outputs: vec![NewPotentialOutputAmount {
                count: 1,
                item: potential_output,
            }],
        };

        // Capture baseline sizes (should be minimal since tx is empty)
        let base_total_size =
            super::estimate_final_tx_total_size(&transaction, inputs_to_sign.as_slice());
        let base_vsize = super::estimate_final_tx_vsize(&transaction, inputs_to_sign.as_slice());

        // Calculate estimated sizes with the additional reserved IOs
        let est_total_size = super::estimate_tx_size_with_additional_inputs_outputs(
            &mut transaction,
            &mut inputs_to_sign,
            &new_potential,
        )
        .unwrap();

        let est_vsize = super::estimate_tx_vsize_with_additional_inputs_outputs(
            &mut transaction,
            &mut inputs_to_sign,
            &new_potential,
        )
        .unwrap();

        // The estimates must be strictly greater than the baseline
        assert!(est_total_size > base_total_size);
        assert!(est_vsize > base_vsize);

        // Ensure that the original transaction and inputs_to_sign remained untouched
        assert_eq!(transaction.input.len(), 0);
        assert_eq!(transaction.output.len(), 0);
        assert_eq!(inputs_to_sign.len(), 0);
    }

    #[test]
    fn test_adjust_transaction_to_pay_fees_success() {
        let mut transaction = create_mock_transaction();
        transaction.output.push(TxOut {
            value: Amount::from_sat(1000),
            script_pubkey: ScriptBuf::new(),
        });

        let inputs_to_sign = vec![create_mock_input_to_sign(0)];
        let total_btc_amount = 2000;
        let address_to_send_remaining_btc =
            Some(create_mock_address(AddressType::User).script_pubkey());
        let fee_rate = FeeRate::try_from(1.0).unwrap();
        let tx_statuses = MempoolInfo::default();

        let result = adjust_transaction_to_pay_fees(
            &mut transaction,
            &inputs_to_sign,
            &tx_statuses,
            total_btc_amount,
            address_to_send_remaining_btc,
            &fee_rate,
        );

        assert!(result.is_ok());
        assert_eq!(transaction.output.len(), 2);
        assert!(transaction.output[1].value.to_sat() < 1000);
    }

    #[test]
    fn test_adjust_transaction_to_pay_fees_not_enough_btc() {
        let mut transaction = create_mock_transaction();
        transaction.output.push(TxOut {
            value: Amount::from_sat(2000),
            script_pubkey: ScriptBuf::new(),
        });

        let inputs_to_sign = vec![create_mock_input_to_sign(0)];
        let total_btc_amount = 2000;
        let address_to_send_remaining_btc =
            Some(create_mock_address(AddressType::User).script_pubkey());
        let fee_rate = FeeRate::try_from(1.0).unwrap();
        let tx_statuses = MempoolInfo::default();

        let result = adjust_transaction_to_pay_fees(
            &mut transaction,
            &inputs_to_sign,
            &tx_statuses,
            total_btc_amount,
            address_to_send_remaining_btc,
            &fee_rate,
        );

        assert_eq!(result, Err(BitcoinTxError::NotEnoughAmountToCoverFees));
    }

    #[test]
    fn test_adjust_transaction_to_pay_fees_no_remaining_address() {
        let mut transaction = create_mock_transaction();
        transaction.output.push(TxOut {
            value: Amount::from_sat(1000),
            script_pubkey: ScriptBuf::new(),
        });

        let inputs_to_sign = vec![create_mock_input_to_sign(0)];
        let total_btc_amount = 2000;
        let address_to_send_remaining_btc: Option<ScriptBuf> = None;
        let fee_rate = FeeRate::try_from(1.0).unwrap();
        let tx_statuses = MempoolInfo::default();

        let result = adjust_transaction_to_pay_fees(
            &mut transaction,
            &inputs_to_sign,
            &tx_statuses,
            total_btc_amount,
            address_to_send_remaining_btc,
            &fee_rate,
        );

        assert!(result.is_ok());
        assert_eq!(transaction.output.len(), 1);
    }

    #[test]
    fn test_adjust_transaction_to_pay_fees_below_dust_limit() {
        let mut transaction = create_mock_transaction();
        transaction.output.push(TxOut {
            value: Amount::from_sat(1990),
            script_pubkey: ScriptBuf::new(),
        });

        let inputs_to_sign = vec![create_mock_input_to_sign(0)];
        let total_btc_amount = 2500;
        let address_to_send_remaining_btc =
            Some(create_mock_address(AddressType::User).script_pubkey());
        let fee_rate = FeeRate::try_from(1.0).unwrap();
        let tx_statuses = MempoolInfo::default();

        let result = adjust_transaction_to_pay_fees(
            &mut transaction,
            &inputs_to_sign,
            &tx_statuses,
            total_btc_amount,
            address_to_send_remaining_btc,
            &fee_rate,
        );

        assert!(result.is_ok());
        assert_eq!(transaction.output.len(), 1);
    }

    #[test]
    fn test_adjust_transaction_to_pay_fees_high_fee_rate() {
        let mut transaction = create_mock_transaction();
        transaction.output.push(TxOut {
            value: Amount::from_sat(1000),
            script_pubkey: ScriptBuf::new(),
        });

        let inputs_to_sign = vec![create_mock_input_to_sign(0)];
        let total_btc_amount = 12000;
        let address_to_send_remaining_btc =
            Some(create_mock_address(AddressType::User).script_pubkey());
        let fee_rate = FeeRate::try_from(100.0).unwrap();
        let tx_statuses = MempoolInfo::default();

        let result = adjust_transaction_to_pay_fees(
            &mut transaction,
            &inputs_to_sign,
            &tx_statuses,
            total_btc_amount,
            address_to_send_remaining_btc,
            &fee_rate,
        );

        assert!(result.is_ok());
        assert_eq!(transaction.output.len(), 1);
    }

    use proptest::prelude::*;

    #[test]
    fn display_witness_weights() {
        use bitcoin::{Transaction, TxIn, TxOut, Witness};

        for inputs in 1..100 {
            // Construct a transaction with 1 input and 1 output
            let mut tx = Transaction {
                version: Version(2),
                lock_time: LockTime::ZERO,
                input: vec![
                    TxIn {
                        previous_output: Default::default(),
                        script_sig: ScriptBuf::new(),
                        sequence: Sequence(0xFFFF_FFFF),
                        witness: Witness::new(), // empty for now
                    };
                    inputs
                ],
                output: vec![TxOut {
                    value: Amount::from_sat(1000),
                    script_pubkey: ScriptBuf::from_hex("6a").unwrap(), // OP_RETURN dummy
                }],
            };

            let inputs_to_sign = (0..inputs)
                .map(|i| InputToSign {
                    index: i as u32,
                    signer: Pubkey([0; 32]),
                })
                .collect::<Vec<_>>();

            let _estimated_total_size = estimate_final_tx_total_size(&tx, &inputs_to_sign);
            let estimated_total_vsize = estimate_final_tx_vsize(&tx, &inputs_to_sign);

            // Now apply actual fake witness to validate correctness
            add_fake_witness_to_transaction(&mut tx, &inputs_to_sign);

            let _final_total_size = tx.total_size();
            let final_total_vsize = tx.vsize();

            println!(
                "Inputs: {} - Real size: {} - Estimated: {}, Diff: {}",
                inputs,
                final_total_vsize,
                estimated_total_vsize,
                (final_total_vsize as isize - estimated_total_vsize as isize)
            );
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 99, ..ProptestConfig::default()
        })]
        #[test]
        fn test_witness_weight_constant(inputs in 1..100_usize) {
            use bitcoin::{Transaction, TxIn, TxOut, Witness};

             // Construct a transaction with 1 input and 1 output
            let mut tx = Transaction {
                version: Version(2),
                lock_time: LockTime::ZERO,
                input: vec![
                    TxIn {
                        previous_output: Default::default(),
                        script_sig: ScriptBuf::new(),
                        sequence: Sequence(0xFFFF_FFFF),
                        witness: Witness::new(), // empty for now
                    };
                    inputs
                ],
                output: vec![TxOut {
                    value: Amount::from_sat(1000),
                    script_pubkey: ScriptBuf::from_hex("6a").unwrap(), // OP_RETURN dummy
                }],
            };

            let inputs_to_sign = (0..inputs)
                .map(|i| InputToSign {
                    index: i as u32,
                    signer: Pubkey([0; 32]),
                })
                .collect::<Vec<_>>();

            let estimated_total_size = estimate_final_tx_total_size(&tx, &inputs_to_sign);
            let estimated_total_vsize = estimate_final_tx_vsize(&tx, &inputs_to_sign) as isize;

            // Now apply actual fake witness to validate correctness
            add_fake_witness_to_transaction(&mut tx, &inputs_to_sign);

            let final_total_size = tx.total_size();
            let final_total_vsize = tx.vsize() as isize;

            assert_eq!(final_total_size, estimated_total_size);

            // Sometimes it's one byte more, sometimes it's one byte less because of rounding
            assert!((final_total_vsize - estimated_total_vsize) <= 1);
        }
    }
}
