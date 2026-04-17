use anchor_lang_v2::prelude::*;

#[error_code]
pub enum OracleError {
    #[msg("signer is not the oracle authority")]
    UnauthorizedAuthority,
}
