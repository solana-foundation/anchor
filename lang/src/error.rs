use anchor_lang::error_code;
use borsh::maybestd::io::Error as BorshIoError;
use solana_program::{program_error::ProgramError, pubkey::Pubkey};
use std::fmt::{Debug, Display};

/// The starting point for user defined error codes.
pub const ERROR_CODE_OFFSET: u32 = 6000;

/// Error codes that can be returned by internal framework code.
///
/// - &gt;= 100 Instruction error codes
/// - &gt;= 1000 IDL error codes
/// - &gt;= 2000 constraint error codes
/// - &gt;= 3000 account error codes
/// - = 4000 state error code
/// - &gt;= 4100 misc error codes
/// - = 5000 deprecated error code
///
/// The starting point for user-defined errors is defined
/// by the [ERROR_CODE_OFFSET](crate::error::ERROR_CODE_OFFSET).
#[error_code(offset = 0)]
pub enum ErrorCode {
    // Instructions
    /// 100 - 8 byte instruction identifier not provided
    #[msg("8 byte instruction identifier not provided")]
    InstructionMissing = 100,
    /// 101 - Fallback functions are not supported
    #[msg("Fallback functions are not supported")]
    InstructionFallbackNotFound,
    /// 102 - The program could not deserialize the given instruction
    #[msg("The program could not deserialize the given instruction")]
    InstructionDidNotDeserialize,
    /// 103 - The program could not serialize the given instruction
    #[msg("The program could not serialize the given instruction")]
    InstructionDidNotSerialize,

    // IDL instructions
    /// 1000 - The program was compiled without idl instructions
    #[msg("The program was compiled without idl instructions")]
    IdlInstructionStub = 1000,
    /// 1001 - Invalid program given to the IDL instruction
    #[msg("Invalid program given to the IDL instruction")]
    IdlInstructionInvalidProgram,

    // Constraints
    /// 2000 - A mut constraint was violated
    #[msg("A mut constraint was violated")]
    ConstraintMut = 2000,
    /// 2001 - A has one constraint was violated
    #[msg("A has one constraint was violated")]
    ConstraintHasOne,
    /// 2002 - A signer constraint was violated
    #[msg("A signer constraint was violated")]
    ConstraintSigner,
    /// 2003 - A raw constraint was violated
    #[msg("A raw constraint was violated")]
    ConstraintRaw,
    /// 2004 - An owner constraint was violated
    #[msg("An owner constraint was violated")]
    ConstraintOwner,
    /// 2005 - A rent exemption constraint was violated
    #[msg("A rent exemption constraint was violated")]
    ConstraintRentExempt,
    /// 2006 - A seeds constraint was violated
    #[msg("A seeds constraint was violated")]
    ConstraintSeeds,
    /// 2007 - An executable constraint was violated
    #[msg("An executable constraint was violated")]
    ConstraintExecutable,
    /// 2008 - A state constraint was violated
    #[msg("A state constraint was violated")]
    ConstraintState,
    /// 2009 - An associated constraint was violated
    #[msg("An associated constraint was violated")]
    ConstraintAssociated,
    /// 2010 - An associated init constraint was violated
    #[msg("An associated init constraint was violated")]
    ConstraintAssociatedInit,
    /// 2011 - A close constraint was violated
    #[msg("A close constraint was violated")]
    ConstraintClose,
    /// 2012 - An address constraint was violated
    #[msg("An address constraint was violated")]
    ConstraintAddress,
    /// 2013 - Expected zero account discriminant
    #[msg("Expected zero account discriminant")]
    ConstraintZero,
    /// 2014 - A token mint constraint was violated
    #[msg("A token mint constraint was violated")]
    ConstraintTokenMint,
    /// 2015 - A token owner constraint was violated
    #[msg("A token owner constraint was violated")]
    ConstraintTokenOwner,
    /// The mint mint is intentional -> a mint authority for the mint.
    ///
    /// 2016 - A mint mint authority constraint was violated
    #[msg("A mint mint authority constraint was violated")]
    ConstraintMintMintAuthority,
    /// 2017 - A mint freeze authority constraint was violated
    #[msg("A mint freeze authority constraint was violated")]
    ConstraintMintFreezeAuthority,
    /// 2018 - A mint decimals constraint was violated
    #[msg("A mint decimals constraint was violated")]
    ConstraintMintDecimals,
    /// 2019 - A space constraint was violated
    #[msg("A space constraint was violated")]
    ConstraintSpace,

    // Require
    /// 2500 - A require expression was violated
    #[msg("A require expression was violated")]
    RequireViolated = 2500,
    /// 2501 - A require_eq expression was violated
    #[msg("A require_eq expression was violated")]
    RequireEqViolated,
    /// 2502 - A require_keys_eq expression was violated
    #[msg("A require_keys_eq expression was violated")]
    RequireKeysEqViolated,
    /// 2503 - A require_neq expression was violated
    #[msg("A require_neq expression was violated")]
    RequireNeqViolated,
    /// 2504 - A require_keys_neq expression was violated
    #[msg("A require_keys_neq expression was violated")]
    RequireKeysNeqViolated,
    /// 2505 - A require_gt expression was violated
    #[msg("A require_gt expression was violated")]
    RequireGtViolated,
    /// 2506 - A require_gte expression was violated
    #[msg("A require_gte expression was violated")]
    RequireGteViolated,

    // Accounts.
    /// 3000 - The account discriminator was already set on this account
    #[msg("The account discriminator was already set on this account")]
    AccountDiscriminatorAlreadySet = 3000,
    /// 3001 - No 8 byte discriminator was found on the account
    #[msg("No 8 byte discriminator was found on the account")]
    AccountDiscriminatorNotFound,
    /// 3002 - 8 byte discriminator did not match what was expected
    #[msg("8 byte discriminator did not match what was expected")]
    AccountDiscriminatorMismatch,
    /// 3003 - Failed to deserialize the account
    #[msg("Failed to deserialize the account")]
    AccountDidNotDeserialize,
    /// 3004 - Failed to serialize the account
    #[msg("Failed to serialize the account")]
    AccountDidNotSerialize,
    /// 3005 - Not enough account keys given to the instruction
    #[msg("Not enough account keys given to the instruction")]
    AccountNotEnoughKeys,
    /// 3006 - The given account is not mutable
    #[msg("The given account is not mutable")]
    AccountNotMutable,
    /// 3007 - The given account is owned by a different program than expected
    #[msg("The given account is owned by a different program than expected")]
    AccountOwnedByWrongProgram,
    /// 3008 - Program ID was not as expected
    #[msg("Program ID was not as expected")]
    InvalidProgramId,
    /// 3009 - Program account is not executable
    #[msg("Program account is not executable")]
    InvalidProgramExecutable,
    /// 3010 - The given account did not sign
    #[msg("The given account did not sign")]
    AccountNotSigner,
    /// 3011 - The given account is not owned by the system program
    #[msg("The given account is not owned by the system program")]
    AccountNotSystemOwned,
    /// 3012 - The program expected this account to be already initialized
    #[msg("The program expected this account to be already initialized")]
    AccountNotInitialized,
    /// 3013 - The given account is not a program data account
    #[msg("The given account is not a program data account")]
    AccountNotProgramData,
    /// 3014 - The given account is not the associated token account
    #[msg("The given account is not the associated token account")]
    AccountNotAssociatedTokenAccount,
    /// 3015 - The given public key does not match the required sysvar
    #[msg("The given public key does not match the required sysvar")]
    AccountSysvarMismatch,

    // State.
    /// 4000 - The given state account does not have the correct address
    #[msg("The given state account does not have the correct address")]
    StateInvalidAddress = 4000,

    // Miscellaneous
    /// 4100 - The declared program id does not match actual program id
    #[msg("The declared program id does not match the actual program id")]
    DeclaredProgramIdMismatch = 4100,

    // Deprecated
    /// 5000 - The API being used is deprecated and should no longer be used
    #[msg("The API being used is deprecated and should no longer be used")]
    Deprecated = 5000,
}

#[derive(Debug)]
pub enum Error {
    AnchorError(AnchorError),
    ProgramError(ProgramErrorWithOrigin),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::AnchorError(ae) => Display::fmt(&ae, f),
            Error::ProgramError(pe) => Display::fmt(&pe, f),
        }
    }
}

impl From<AnchorError> for Error {
    fn from(ae: AnchorError) -> Self {
        Self::AnchorError(ae)
    }
}

impl From<ProgramError> for Error {
    fn from(program_error: ProgramError) -> Self {
        Self::ProgramError(program_error.into())
    }
}
impl From<BorshIoError> for Error {
    fn from(error: BorshIoError) -> Self {
        Error::ProgramError(ProgramError::from(error).into())
    }
}

impl From<ProgramErrorWithOrigin> for Error {
    fn from(pe: ProgramErrorWithOrigin) -> Self {
        Self::ProgramError(pe)
    }
}

impl Error {
    pub fn log(&self) {
        match self {
            Error::ProgramError(program_error) => program_error.log(),
            Error::AnchorError(anchor_error) => anchor_error.log(),
        }
    }

    pub fn with_account_name(mut self, account_name: impl ToString) -> Self {
        match &mut self {
            Error::AnchorError(ae) => {
                ae.error_origin = Some(ErrorOrigin::AccountName(account_name.to_string()));
            }
            Error::ProgramError(pe) => {
                pe.error_origin = Some(ErrorOrigin::AccountName(account_name.to_string()));
            }
        };
        self
    }

    pub fn with_source(mut self, source: Source) -> Self {
        match &mut self {
            Error::AnchorError(ae) => {
                ae.error_origin = Some(ErrorOrigin::Source(source));
            }
            Error::ProgramError(pe) => {
                pe.error_origin = Some(ErrorOrigin::Source(source));
            }
        };
        self
    }

    pub fn with_pubkeys(mut self, pubkeys: (Pubkey, Pubkey)) -> Self {
        let pubkeys = Some(ComparedValues::Pubkeys((pubkeys.0, pubkeys.1)));
        match &mut self {
            Error::AnchorError(ae) => ae.compared_values = pubkeys,
            Error::ProgramError(pe) => pe.compared_values = pubkeys,
        };
        self
    }

    pub fn with_values(mut self, values: (impl ToString, impl ToString)) -> Self {
        match &mut self {
            Error::AnchorError(ae) => {
                ae.compared_values = Some(ComparedValues::Values((
                    values.0.to_string(),
                    values.1.to_string(),
                )))
            }
            Error::ProgramError(pe) => {
                pe.compared_values = Some(ComparedValues::Values((
                    values.0.to_string(),
                    values.1.to_string(),
                )))
            }
        };
        self
    }
}

#[derive(Debug)]
pub struct ProgramErrorWithOrigin {
    pub program_error: ProgramError,
    pub error_origin: Option<ErrorOrigin>,
    pub compared_values: Option<ComparedValues>,
}

impl Display for ProgramErrorWithOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.program_error, f)
    }
}

impl ProgramErrorWithOrigin {
    pub fn log(&self) {
        match &self.error_origin {
            None => {
                anchor_lang::solana_program::msg!(
                    "ProgramError occurred. Error Code: {:?}. Error Number: {}. Error Message: {}.",
                    self.program_error,
                    u64::from(self.program_error.clone()),
                    self.program_error
                );
            }
            Some(ErrorOrigin::Source(source)) => {
                anchor_lang::solana_program::msg!(
                    "ProgramError thrown in {}:{}. Error Code: {:?}. Error Number: {}. Error Message: {}.",
                    source.filename,
                    source.line,
                    self.program_error,
                    u64::from(self.program_error.clone()),
                    self.program_error
                );
            }
            Some(ErrorOrigin::AccountName(account_name)) => {
                // using sol_log because msg! wrongly interprets 5 inputs as u64
                anchor_lang::solana_program::log::sol_log(&format!(
                    "ProgramError caused by account: {}. Error Code: {:?}. Error Number: {}. Error Message: {}.",
                    account_name,
                    self.program_error,
                    u64::from(self.program_error.clone()),
                    self.program_error
                ));
            }
        }
        match &self.compared_values {
            Some(ComparedValues::Pubkeys((left, right))) => {
                anchor_lang::solana_program::msg!("Left:");
                left.log();
                anchor_lang::solana_program::msg!("Right:");
                right.log();
            }
            Some(ComparedValues::Values((left, right))) => {
                anchor_lang::solana_program::msg!("Left: {}", left);
                anchor_lang::solana_program::msg!("Right: {}", right);
            }
            None => (),
        }
    }

    pub fn with_source(mut self, source: Source) -> Self {
        self.error_origin = Some(ErrorOrigin::Source(source));
        self
    }

    pub fn with_account_name(mut self, account_name: impl ToString) -> Self {
        self.error_origin = Some(ErrorOrigin::AccountName(account_name.to_string()));
        self
    }
}

impl From<ProgramError> for ProgramErrorWithOrigin {
    fn from(program_error: ProgramError) -> Self {
        Self {
            program_error,
            error_origin: None,
            compared_values: None,
        }
    }
}

#[derive(Debug)]
pub enum ComparedValues {
    Values((String, String)),
    Pubkeys((Pubkey, Pubkey)),
}

#[derive(Debug)]
pub enum ErrorOrigin {
    Source(Source),
    AccountName(String),
}

#[derive(Debug)]
pub struct AnchorError {
    pub error_name: String,
    pub error_code_number: u32,
    pub error_msg: String,
    pub error_origin: Option<ErrorOrigin>,
    pub compared_values: Option<ComparedValues>,
}

impl AnchorError {
    pub fn log(&self) {
        match &self.error_origin {
            None => {
                anchor_lang::solana_program::log::sol_log(&format!(
                    "AnchorError occurred. Error Code: {}. Error Number: {}. Error Message: {}.",
                    self.error_name, self.error_code_number, self.error_msg
                ));
            }
            Some(ErrorOrigin::Source(source)) => {
                anchor_lang::solana_program::msg!(
                    "AnchorError thrown in {}:{}. Error Code: {}. Error Number: {}. Error Message: {}.",
                    source.filename,
                    source.line,
                    self.error_name,
                    self.error_code_number,
                    self.error_msg
                );
            }
            Some(ErrorOrigin::AccountName(account_name)) => {
                anchor_lang::solana_program::log::sol_log(&format!(
                    "AnchorError caused by account: {}. Error Code: {}. Error Number: {}. Error Message: {}.",
                    account_name,
                    self.error_name,
                    self.error_code_number,
                    self.error_msg
                ));
            }
        }
        match &self.compared_values {
            Some(ComparedValues::Pubkeys((left, right))) => {
                anchor_lang::solana_program::msg!("Left:");
                left.log();
                anchor_lang::solana_program::msg!("Right:");
                right.log();
            }
            Some(ComparedValues::Values((left, right))) => {
                anchor_lang::solana_program::msg!("Left: {}", left);
                anchor_lang::solana_program::msg!("Right: {}", right);
            }
            None => (),
        }
    }

    pub fn with_source(mut self, source: Source) -> Self {
        self.error_origin = Some(ErrorOrigin::Source(source));
        self
    }

    pub fn with_account_name(mut self, account_name: impl ToString) -> Self {
        self.error_origin = Some(ErrorOrigin::AccountName(account_name.to_string()));
        self
    }
}

impl Display for AnchorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

impl std::convert::From<Error> for anchor_lang::solana_program::program_error::ProgramError {
    fn from(e: Error) -> anchor_lang::solana_program::program_error::ProgramError {
        match e {
            Error::AnchorError(AnchorError {
                error_code_number, ..
            }) => {
                anchor_lang::solana_program::program_error::ProgramError::Custom(error_code_number)
            }
            Error::ProgramError(program_error) => program_error.program_error,
        }
    }
}

#[derive(Debug)]
pub struct Source {
    pub filename: &'static str,
    pub line: u32,
}
