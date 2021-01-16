use solana_program::program_error::ProgramError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ProgramError(#[from] ProgramError),
    #[error("{0:?}")]
    ErrorCode(#[from] ErrorCode),
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum ErrorCode {
    WrongSerialization = 1,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        <Self as std::fmt::Debug>::fmt(self, fmt)
    }
}

impl std::error::Error for ErrorCode {}

impl std::convert::From<Error> for ProgramError {
    fn from(e: Error) -> ProgramError {
        match e {
            Error::ProgramError(e) => e,
            Error::ErrorCode(c) => ProgramError::Custom(c as u32),
        }
    }
}
