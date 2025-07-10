//! A trait for sanitizing values and members of over the wire messages.

use {crate::pubkey::Pubkey, bitcoin::hex::DisplayHex, core::fmt, std::error::Error};

#[derive(PartialEq, Debug, Eq, Clone)]
pub enum SanitizeError {
    IndexOutOfBounds,
    ValueOutOfBounds,
    InvalidValue,
    InvalidVersion,
    SignatureCountMismatch { expected: usize, actual: usize },
    InvalidRecentBlockhash,
    DuplicateAccount,
}

impl Error for SanitizeError {}

impl fmt::Display for SanitizeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SanitizeError::IndexOutOfBounds => f.write_str("index out of bounds"),
            SanitizeError::ValueOutOfBounds => f.write_str("value out of bounds"),
            SanitizeError::InvalidValue => f.write_str("invalid value"),
            SanitizeError::InvalidVersion => f.write_str("invalid version"),
            SanitizeError::SignatureCountMismatch { expected, actual } => {
                write!(
                    f,
                    "signature count mismatch: expected {}, actual {}",
                    expected, actual
                )
            }
            SanitizeError::InvalidRecentBlockhash => f.write_str("invalid recent blockhash"),
            SanitizeError::DuplicateAccount => f.write_str("duplicate accounts detected"),
        }
    }
}

/// A trait for sanitizing values and members of over-the-wire messages.
///
/// Implementation should recursively descend through the data structure and
/// sanitize all struct members and enum clauses. Sanitize excludes signature-
/// verification checks, those are handled by another pass. Sanitize checks
/// should include but are not limited to:
///
/// - All index values are in range.
/// - All values are within their static max/min bounds.
pub trait Sanitize {
    fn sanitize(&self) -> Result<(), SanitizeError> {
        Ok(())
    }
}

impl<T: Sanitize> Sanitize for Vec<T> {
    fn sanitize(&self) -> Result<(), SanitizeError> {
        for x in self.iter() {
            x.sanitize()?;
        }
        Ok(())
    }
}
