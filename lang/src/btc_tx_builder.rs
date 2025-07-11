//! Type-erased access to `satellite_bitcoin::TransactionBuilder`.
//!
//! The `BtcTxBuilderAny` trait provides an object-safe façade exposing the most
//! useful methods of `TransactionBuilder` so that user handlers can work with a
//! simple `&mut dyn BtcTxBuilderAny` rather than needing to know the exact
//! generic parameters chosen by the code generator.

use arch_program::account::AccountInfo;
use arch_program::program_error::ProgramError;
use satellite_bitcoin::fee_rate::FeeRate;
use bitcoin::ScriptBuf;

// Types referenced by the signature of forwarded methods --------------------
use satellite_bitcoin::NewPotentialInputsAndOutputs;
use satellite_bitcoin::generic::fixed_set::FixedCapacitySet;
use arch_program::rune::RuneAmount;

/// Object-safe subset of [`satellite_bitcoin::TransactionBuilder`].
///
/// All forwarding methods must stay *object-safe* so that a `dyn
/// BtcTxBuilderAny` can be created.  Do **not** add generic methods here.
pub trait BtcTxBuilderAny<'info> {
    /// Add a state-transition input for the given Solana account.
    fn add_state_transition(
        &mut self,
        account: &AccountInfo<'info>,
    ) -> Result<(), ProgramError>;

    /// Insert a state-transition input at a specific transaction index.
    fn insert_state_transition_input(
        &mut self,
        tx_index: usize,
        account: &AccountInfo<'info>,
    ) -> Result<(), ProgramError>;

    /// Ensure the transaction pays adequate fees at `fee_rate`.
    fn adjust_transaction_to_pay_fees(
        &mut self,
        fee_rate: &FeeRate,
        address_to_send_remaining_btc: Option<ScriptBuf>,
    ) -> Result<(), ProgramError>;

    fn get_fee_paid_by_program(&self, fee_rate: &FeeRate) -> u64;
    fn get_fee_paid_by_user(&mut self, fee_rate: &FeeRate) -> u64;

    fn estimate_final_tx_vsize(&mut self) -> usize;

    fn estimate_tx_size_with_additional_inputs_outputs(
        &mut self,
        new_ios: &NewPotentialInputsAndOutputs,
    ) -> Result<usize, ProgramError>;

    fn estimate_tx_vsize_with_additional_inputs_outputs(
        &mut self,
        new_ios: &NewPotentialInputsAndOutputs,
    ) -> Result<usize, ProgramError>;

    fn get_ancestors_totals(&self) -> Result<(usize, u64), ProgramError>;
    fn get_fee_paid(&self) -> Result<u64, ProgramError>;

    fn is_fee_rate_valid(&mut self, fee_rate: &FeeRate) -> Result<(), ProgramError>;

    /// Finalise the transaction so that the runtime can collect signatures.
    fn finalize(&mut self) -> Result<(), ProgramError>;
}

// Blanket implementation for every `TransactionBuilder` variant -------------

impl<'info, const MAX_MODIFIED_ACCOUNTS: usize, const MAX_INPUTS_TO_SIGN: usize, RuneSet>
    BtcTxBuilderAny<'info>
    for satellite_bitcoin::TransactionBuilder<
        'info,
        MAX_MODIFIED_ACCOUNTS,
        MAX_INPUTS_TO_SIGN,
        RuneSet,
    >
where
    RuneSet: FixedCapacitySet<Item = RuneAmount> + Default,
{
    #[inline]
    fn add_state_transition(
        &mut self,
        account: &AccountInfo<'info>,
    ) -> Result<(), ProgramError> {
        self.add_state_transition(account).map_err(|e| e.into())
    }

    #[inline]
    fn insert_state_transition_input(
        &mut self,
        tx_index: usize,
        account: &AccountInfo<'info>,
    ) -> Result<(), ProgramError> {
        self.insert_state_transition_input(tx_index, account)
            .map_err(|e| e.into())
    }

    #[inline]
    fn adjust_transaction_to_pay_fees(
        &mut self,
        fee_rate: &FeeRate,
        address_to_send_remaining_btc: Option<ScriptBuf>,
    ) -> Result<(), ProgramError> {
        self.adjust_transaction_to_pay_fees(fee_rate, address_to_send_remaining_btc)
            .map_err(|e| e.into())
    }

    #[inline]
    fn get_fee_paid_by_program(&self, fee_rate: &FeeRate) -> u64 {
        self.get_fee_paid_by_program(fee_rate)
    }

    #[inline]
    fn get_fee_paid_by_user(&mut self, fee_rate: &FeeRate) -> u64 {
        self.get_fee_paid_by_user(fee_rate)
    }

    #[inline]
    fn estimate_final_tx_vsize(&mut self) -> usize {
        self.estimate_final_tx_vsize()
    }

    #[inline]
    fn estimate_tx_size_with_additional_inputs_outputs(
        &mut self,
        new_ios: &NewPotentialInputsAndOutputs,
    ) -> Result<usize, ProgramError> {
        self.estimate_tx_size_with_additional_inputs_outputs(new_ios)
            .map_err(|e| e.into())
    }

    #[inline]
    fn estimate_tx_vsize_with_additional_inputs_outputs(
        &mut self,
        new_ios: &NewPotentialInputsAndOutputs,
    ) -> Result<usize, ProgramError> {
        self.estimate_tx_vsize_with_additional_inputs_outputs(new_ios)
            .map_err(|e| e.into())
    }

    #[inline]
    fn get_ancestors_totals(&self) -> Result<(usize, u64), ProgramError> {
        self.get_ancestors_totals().map_err(|e| e.into())
    }

    #[inline]
    fn get_fee_paid(&self) -> Result<u64, ProgramError> {
        self.get_fee_paid().map_err(|e| e.into())
    }

    #[inline]
    fn is_fee_rate_valid(&mut self, fee_rate: &FeeRate) -> Result<(), ProgramError> {
        self.is_fee_rate_valid(fee_rate).map_err(|e| e.into())
    }

    #[inline]
    fn finalize(&mut self) -> Result<(), ProgramError> {
        self.finalize()
    }
} 