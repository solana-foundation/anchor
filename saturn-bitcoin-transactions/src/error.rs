use saturn_collections::generic::fixed_set::FixedSetError;
use anchor_lang::error_code;
use saturn_safe_math::MathError;

#[error_code(offset = 800)]
pub enum BitcoinTxError {
    #[msg("Transaction input amount is not enough to cover network fees")]
    NotEnoughAmountToCoverFees,

    #[msg("The resulting transaction exceeds the maximum size allowed")]
    TransactionTooLarge,

    #[msg("An arithmetic error ocurred")]
    CalcOverflow,

    #[msg("The transaction inputs don't cover the amount to be spent in the transaction")]
    InsufficientInputAmount,

    #[msg("The configured fee rate is too low")]
    InvalidFeeRateTooLow,

    #[msg("The utxo was not found in the user utxos")]
    UtxoNotFoundInUserUtxos,

    #[msg("The transaction input length must match the user utxos length")]
    TransactionInputLengthMustMatchUserUtxosLength,

    #[msg("The transaction was not found")]
    TransactionNotFound,

    #[msg("The utxo does not contain runes")]
    RuneOutputNotFound,

    #[msg("The utxo contains more runes than the maximum allowed")]
    MoreRunesInUtxoThanMax,

    #[msg("Not enough BTC in pool")]
    NotEnoughBtcInPool,

    #[msg("The runestone is not valid")]
    RunestoneDecipherError,

    #[msg("Rune input list is full")]
    RuneInputListFull,

    #[msg("Rune addition overflow")]
    RuneAdditionOverflow,

    #[msg("Input to sign list is full")]
    InputToSignListFull,

    #[msg("Modified account list is full")]
    ModifiedAccountListFull,
}

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
            MathError::AdditionOverflow => BitcoinTxError::CalcOverflow,
            MathError::SubtractionOverflow => BitcoinTxError::CalcOverflow,
            MathError::MultiplicationOverflow => BitcoinTxError::CalcOverflow,
            MathError::DivisionOverflow => BitcoinTxError::CalcOverflow,
            MathError::ConversionError => BitcoinTxError::CalcOverflow,
        }
    }
}
