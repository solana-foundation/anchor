use arch_program::{decode_error::DecodeError, program_error::ProgramError};
use num_derive::FromPrimitive;
use thiserror::Error;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error, FromPrimitive)]
pub enum ErrorCode {
    #[error("Required UTXO matching the predicate was not found")]
    MissingRequiredUtxo = 900,
    #[error("There are leftover UTXOs that were not consumed by the parser")]
    UnexpectedExtraUtxos,
    #[error("UTXO value (satoshis) did not match the expected value")]
    InvalidUtxoValue,
    #[error("UTXO runes presence (none/some) did not match expectation")]
    InvalidRunesPresence,
    #[error("Required rune id was not found in the UTXO")]
    InvalidRuneId,
    #[error("Rune amount in UTXO did not match expectation")]
    InvalidRuneAmount,
    #[error("Duplicate UTXO meta in the provided inputs list")]
    DuplicateUtxoMeta,
    #[error("UTXO did not satisfy the expected predicate at its strict-order position")]
    StrictOrderMismatch,
}

// === Conversions ============================================================

impl From<ErrorCode> for ProgramError {
    fn from(e: ErrorCode) -> Self {
        ProgramError::Custom(e.into())
    }
}

/// Allow using `.into()` to convert directly into the underlying `u32` code.
impl From<ErrorCode> for u32 {
    fn from(e: ErrorCode) -> Self {
        e as u32
    }
}

// === Runtime decoding support ==============================================

impl DecodeError<ErrorCode> for ErrorCode {
    fn type_of() -> &'static str {
        "ErrorCode"
    }
}
