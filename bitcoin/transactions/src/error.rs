use satellite_collections::generic::fixed_set::FixedSetError;
use satellite_math::MathError;
use arch_program::program_error::ProgramError;
use arch_program::decode_error::DecodeError;
use num_derive::FromPrimitive;
use thiserror::Error;

/// Custom errors for Bitcoin transaction operations.
///
/// The first variant starts at 800 to maintain compatibility with the previous
/// Anchor‐based error numbering.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error, FromPrimitive)]
pub enum BitcoinTxError {
    #[error("Transaction input amount is not enough to cover network fees")]
    NotEnoughAmountToCoverFees = 800,

    #[error("The resulting transaction exceeds the maximum size allowed")]
    TransactionTooLarge,

    #[error("An arithmetic error ocurred")]
    CalcOverflow,

    #[error("The transaction inputs don't cover the amount to be spent in the transaction")]
    InsufficientInputAmount,

    #[error("The configured fee rate is too low")]
    InvalidFeeRateTooLow,

    #[error("The utxo was not found in the user utxos")]
    UtxoNotFoundInUserUtxos,

    #[error("The transaction input length must match the user utxos length")]
    TransactionInputLengthMustMatchUserUtxosLength,

    #[error("The transaction was not found")]
    TransactionNotFound,

    #[error("The utxo does not contain runes")]
    RuneOutputNotFound,

    #[error("The utxo contains more runes than the maximum allowed")]
    MoreRunesInUtxoThanMax,

    #[error("Not enough BTC in pool")]
    NotEnoughBtcInPool,

    #[error("The runestone is not valid")]
    RunestoneDecipherError,

    #[error("Rune input list is full")]
    RuneInputListFull,

    #[error("Rune addition overflow")]
    RuneAdditionOverflow,

    #[error("Input to sign list is full")]
    InputToSignListFull,

    #[error("Modified account list is full")]
    ModifiedAccountListFull,
}

// === Conversions ============================================================

impl From<FixedSetError> for BitcoinTxError {
    fn from(error: FixedSetError) -> Self {
        match error {
            FixedSetError::Full => BitcoinTxError::RuneInputListFull,
            FixedSetError::Duplicate => panic!("Duplicate rune input"),
        }
    }
}

impl From<MathError> for BitcoinTxError {
    fn from(error: MathError) -> Self {
        match error {
            MathError::AdditionOverflow
            | MathError::SubtractionOverflow
            | MathError::MultiplicationOverflow
            | MathError::DivisionOverflow
            | MathError::ConversionError => BitcoinTxError::CalcOverflow,
        }
    }
}

/// Convert to the generic [`ProgramError`] used by `arch_program`.
impl From<BitcoinTxError> for ProgramError {
    fn from(e: BitcoinTxError) -> Self {
        ProgramError::Custom(e.into())
    }
}

/// Allow using `.into()` to convert directly into the underlying `u32` code.
impl From<BitcoinTxError> for u32 {
    fn from(e: BitcoinTxError) -> Self {
        e as u32
    }
}

// === Runtime decoding support ==============================================

impl DecodeError<BitcoinTxError> for BitcoinTxError {
    fn type_of() -> &'static str {
        "BitcoinTxError"
    }
}
