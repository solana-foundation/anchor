use crate::error;

/// Error codes that can be returned by internal framework code.
///
/// - &gt;= 100 Instruction error codes
/// - &gt;= 1000 IDL error codes
/// - &gt;= 2000 constraint error codes
/// - &gt;= 3000 account error codes
/// - = 4000 state error code
/// - = 5000 deprecated error code
///
/// The starting point for user-defined errors is defined
/// by the [ERROR_CODE_OFFSET](crate::__private::ERROR_CODE_OFFSET).
#[error(offset = 0)]
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

    // State.
    /// 4000 - The given state account does not have the correct address
    #[msg("The given state account does not have the correct address")]
    StateInvalidAddress = 4000,

    // Deprecated
    /// 5000 - The API being used is deprecated and should no longer be used
    #[msg("The API being used is deprecated and should no longer be used")]
    Deprecated = 5000,
}
