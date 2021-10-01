export class IdlError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "IdlError";
  }
}

// An error from a user defined program.
export class ProgramError extends Error {
  constructor(readonly code: number, readonly msg: string, ...params: any[]) {
    super(...params);
  }

  public static parse(
    err: any,
    idlErrors: Map<number, string>
  ): ProgramError | null {
    // TODO: don't rely on the error string. web3.js should preserve the error
    //       code information instead of giving us an untyped string.
    let components = err.toString().split("custom program error: ");
    if (components.length !== 2) {
      return null;
    }

    let errorCode: number;
    try {
      errorCode = parseInt(components[1]);
    } catch (parseErr) {
      return null;
    }

    // Parse user error.
    let errorMsg = idlErrors.get(errorCode);
    if (errorMsg !== undefined) {
      return new ProgramError(errorCode, errorMsg, errorCode + ": " + errorMsg);
    }

    // Parse framework internal error.
    errorMsg = LangErrorMessage.get(errorCode);
    if (errorMsg !== undefined) {
      return new ProgramError(errorCode, errorMsg, errorCode + ": " + errorMsg);
    }

    // Unable to parse the error. Just return the untranslated error.
    return null;
  }

  public toString(): string {
    return this.msg;
  }
}

const LangErrorCode = {
  // Instructions.
  InstructionMissing: 100,
  InstructionFallbackNotFound: 101,
  InstructionDidNotDeserialize: 102,
  InstructionDidNotSerialize: 103,

  // IDL instructions.
  IdlInstructionStub: 120,
  IdlInstructionInvalidProgram: 121,

  // Constraints.
  ConstraintMut: 140,
  ConstraintHasOne: 141,
  ConstraintSigner: 142,
  ConstraintRaw: 143,
  ConstraintOwner: 144,
  ConstraintRentExempt: 145,
  ConstraintSeeds: 146,
  ConstraintExecutable: 147,
  ConstraintState: 148,
  ConstraintAssociated: 149,
  ConstraintAssociatedInit: 150,
  ConstraintClose: 151,
  ConstraintAddress: 152,

  // Accounts.
  AccountDiscriminatorAlreadySet: 160,
  AccountDiscriminatorNotFound: 161,
  AccountDiscriminatorMismatch: 162,
  AccountDidNotDeserialize: 163,
  AccountDidNotSerialize: 164,
  AccountNotEnoughKeys: 165,
  AccountNotMutable: 166,
  AccountNotProgramOwned: 167,
  InvalidProgramId: 168,
  InvalidProgramIdExecutable: 169,

  // State.
  StateInvalidAddress: 180,

  // Used for APIs that shouldn't be used anymore.
  Deprecated: 299,
};

const LangErrorMessage = new Map([
  // Instructions.
  [
    LangErrorCode.InstructionMissing,
    "8 byte instruction identifier not provided",
  ],
  [
    LangErrorCode.InstructionFallbackNotFound,
    "Fallback functions are not supported",
  ],
  [
    LangErrorCode.InstructionDidNotDeserialize,
    "The program could not deserialize the given instruction",
  ],
  [
    LangErrorCode.InstructionDidNotSerialize,
    "The program could not serialize the given instruction",
  ],

  // Idl instructions.
  [
    LangErrorCode.IdlInstructionStub,
    "The program was compiled without idl instructions",
  ],
  [
    LangErrorCode.IdlInstructionInvalidProgram,
    "The transaction was given an invalid program for the IDL instruction",
  ],

  // Constraints.
  [LangErrorCode.ConstraintMut, "A mut constraint was violated"],
  [LangErrorCode.ConstraintHasOne, "A has_one constraint was violated"],
  [LangErrorCode.ConstraintSigner, "A signer constraint was violated"],
  [LangErrorCode.ConstraintRaw, "A raw constraint was violated"],
  [LangErrorCode.ConstraintOwner, "An owner constraint was violated"],
  [LangErrorCode.ConstraintRentExempt, "A rent exempt constraint was violated"],
  [LangErrorCode.ConstraintSeeds, "A seeds constraint was violated"],
  [LangErrorCode.ConstraintExecutable, "An executable constraint was violated"],
  [LangErrorCode.ConstraintState, "A state constraint was violated"],
  [LangErrorCode.ConstraintAssociated, "An associated constraint was violated"],
  [
    LangErrorCode.ConstraintAssociatedInit,
    "An associated init constraint was violated",
  ],
  [LangErrorCode.ConstraintClose, "A close constraint was violated"],
  [LangErrorCode.ConstraintAddress, "An address constraint was violated"],

  // Accounts.
  [
    LangErrorCode.AccountDiscriminatorAlreadySet,
    "The account discriminator was already set on this account",
  ],
  [
    LangErrorCode.AccountDiscriminatorNotFound,
    "No 8 byte discriminator was found on the account",
  ],
  [
    LangErrorCode.AccountDiscriminatorMismatch,
    "8 byte discriminator did not match what was expected",
  ],
  [LangErrorCode.AccountDidNotDeserialize, "Failed to deserialize the account"],
  [LangErrorCode.AccountDidNotSerialize, "Failed to serialize the account"],
  [
    LangErrorCode.AccountNotEnoughKeys,
    "Not enough account keys given to the instruction",
  ],
  [LangErrorCode.AccountNotMutable, "The given account is not mutable"],
  [
    LangErrorCode.AccountNotProgramOwned,
    "The given account is not owned by the executing program",
  ],
  [LangErrorCode.InvalidProgramId, "Program ID was not as expected"],
  [
    LangErrorCode.InvalidProgramIdExecutable,
    "Program account is not executable",
  ],

  // State.
  [
    LangErrorCode.StateInvalidAddress,
    "The given state account does not have the correct address",
  ],

  // Misc.
  [
    LangErrorCode.Deprecated,
    "The API being used is deprecated and should no longer be used",
  ],
]);
