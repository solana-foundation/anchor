use arch_program::program_error::ProgramError;

#[derive(Debug)]
pub enum ErrorCode {
    MempoolCPIError = 900,
    InvalidMempoolData = 901,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Into<u32> for ErrorCode {
    fn into(self) -> u32 {
        self as u32
    }
}

impl Into<ProgramError> for ErrorCode {
    fn into(self) -> ProgramError {
        ProgramError::Custom(self.into())
    }
}
