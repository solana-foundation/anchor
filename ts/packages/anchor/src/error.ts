import { PublicKey } from "@solana/web3.js";
import * as features from "./utils/features.js";

export class IdlError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "IdlError";
  }
}

interface ErrorCode {
  code: string;
  number: number;
}

interface FileLine {
  file: string;
  line: number;
}

type Origin = string | FileLine;
type ComparedAccountNames = [string, string];
type ComparedPublicKeys = [PublicKey, PublicKey];
type ComparedValues = ComparedAccountNames | ComparedPublicKeys;

export class ProgramErrorStack {
  constructor(readonly stack: PublicKey[]) {}

  public static parse(logs: string[]) {
    const programKeyRegex = /^Program (\w*) invoke/;
    const successRegex = /^Program \w* success/;

    const programStack: PublicKey[] = [];
    for (let i = 0; i < logs.length; i++) {
      if (successRegex.exec(logs[i])) {
        programStack.pop();
        continue;
      }

      const programKey = programKeyRegex.exec(logs[i])?.[1];
      if (!programKey) {
        continue;
      }
      programStack.push(new PublicKey(programKey));
    }
    return new ProgramErrorStack(programStack);
  }
}

export class AnchorError extends Error {
  readonly error: {
    errorCode: ErrorCode;
    errorMessage: string;
    comparedValues?: ComparedValues;
    origin?: Origin;
  };
  private readonly _programErrorStack: ProgramErrorStack;

  constructor(
    errorCode: ErrorCode,
    errorMessage: string,
    readonly errorLogs: string[],
    readonly logs: string[],
    origin?: Origin,
    comparedValues?: ComparedValues
  ) {
    super(errorLogs.join("\n").replace("Program log: ", ""));
    this.error = { errorCode, errorMessage, comparedValues, origin };
    this._programErrorStack = ProgramErrorStack.parse(logs);
  }

  public static parse(logs: string[]) {
    if (!logs) {
      return null;
    }

    const anchorErrorLogIndex = logs.findIndex((log) =>
      log.startsWith("Program log: AnchorError")
    );
    if (anchorErrorLogIndex === -1) {
      return null;
    }
    const anchorErrorLog = logs[anchorErrorLogIndex];
    const errorLogs = [anchorErrorLog];
    let comparedValues: ComparedValues | undefined;
    if (anchorErrorLogIndex + 1 < logs.length) {
      // This catches the comparedValues where the following is logged
      // <AnchorError>
      // Left:
      // <Pubkey>
      // Right:
      // <Pubkey>
      if (logs[anchorErrorLogIndex + 1] === "Program log: Left:") {
        const pubkeyRegex = /^Program log: (.*)$/;
        const leftPubkey = pubkeyRegex.exec(logs[anchorErrorLogIndex + 2])![1];
        const rightPubkey = pubkeyRegex.exec(logs[anchorErrorLogIndex + 4])![1];
        comparedValues = [
          new PublicKey(leftPubkey),
          new PublicKey(rightPubkey),
        ];
        errorLogs.push(
          ...logs.slice(anchorErrorLogIndex + 1, anchorErrorLogIndex + 5)
        );
      }
      // This catches the comparedValues where the following is logged
      // <AnchorError>
      // Left: <value>
      // Right: <value>
      else if (logs[anchorErrorLogIndex + 1].startsWith("Program log: Left:")) {
        const valueRegex = /^Program log: (Left|Right): (.*)$/;
        const leftValue = valueRegex.exec(logs[anchorErrorLogIndex + 1])![2];
        const rightValue = valueRegex.exec(logs[anchorErrorLogIndex + 2])![2];
        errorLogs.push(
          ...logs.slice(anchorErrorLogIndex + 1, anchorErrorLogIndex + 3)
        );
        comparedValues = [leftValue, rightValue];
      }
    }
    const regexNoInfo =
      /^Program log: AnchorError occurred\. Error Code: (.*)\. Error Number: (\d*)\. Error Message: (.*)\./;
    const noInfoAnchorErrorLog = regexNoInfo.exec(anchorErrorLog);
    const regexFileLine =
      /^Program log: AnchorError thrown in (.*):(\d*)\. Error Code: (.*)\. Error Number: (\d*)\. Error Message: (.*)\./;
    const fileLineAnchorErrorLog = regexFileLine.exec(anchorErrorLog);
    const regexAccountName =
      /^Program log: AnchorError caused by account: (.*)\. Error Code: (.*)\. Error Number: (\d*)\. Error Message: (.*)\./;
    const accountNameAnchorErrorLog = regexAccountName.exec(anchorErrorLog);
    if (noInfoAnchorErrorLog) {
      const [errorCodeString, errorNumber, errorMessage] =
        noInfoAnchorErrorLog.slice(1, 4);
      const errorCode = {
        code: errorCodeString,
        number: parseInt(errorNumber),
      };
      return new AnchorError(
        errorCode,
        errorMessage,
        errorLogs,
        logs,
        undefined,
        comparedValues
      );
    } else if (fileLineAnchorErrorLog) {
      const [file, line, errorCodeString, errorNumber, errorMessage] =
        fileLineAnchorErrorLog.slice(1, 6);
      const errorCode = {
        code: errorCodeString,
        number: parseInt(errorNumber),
      };
      const fileLine = { file, line: parseInt(line) };
      return new AnchorError(
        errorCode,
        errorMessage,
        errorLogs,
        logs,
        fileLine,
        comparedValues
      );
    } else if (accountNameAnchorErrorLog) {
      const [accountName, errorCodeString, errorNumber, errorMessage] =
        accountNameAnchorErrorLog.slice(1, 5);
      const origin = accountName;
      const errorCode = {
        code: errorCodeString,
        number: parseInt(errorNumber),
      };
      return new AnchorError(
        errorCode,
        errorMessage,
        errorLogs,
        logs,
        origin,
        comparedValues
      );
    } else {
      return null;
    }
  }

  get program(): PublicKey {
    return this._programErrorStack.stack[
      this._programErrorStack.stack.length - 1
    ];
  }

  get programErrorStack(): PublicKey[] {
    return this._programErrorStack.stack;
  }

  public toString(): string {
    return this.message;
  }
}

// An error from a user defined program.
export class ProgramError extends Error {
  private readonly _programErrorStack?: ProgramErrorStack;

  constructor(
    readonly code: number,
    readonly msg: string,
    readonly logs?: string[]
  ) {
    super();
    if (logs) {
      this._programErrorStack = ProgramErrorStack.parse(logs);
    }
  }

  public static parse(
    err: any,
    idlErrors: Map<number, string>
  ): ProgramError | null {
    const errString: string = err.toString();
    // TODO: don't rely on the error string. web3.js should preserve the error
    //       code information instead of giving us an untyped string.
    let unparsedErrorCode: string;
    if (errString.includes("custom program error:")) {
      let components = errString.split("custom program error: ");
      if (components.length !== 2) {
        return null;
      } else {
        unparsedErrorCode = components[1];
      }
    } else {
      const matches = errString.match(/"Custom":([0-9]+)}/g);
      if (!matches || matches.length > 1) {
        return null;
      }
      unparsedErrorCode = matches[0].match(/([0-9]+)/g)![0];
    }

    let errorCode: number;
    try {
      errorCode = parseInt(unparsedErrorCode);
    } catch (parseErr) {
      return null;
    }

    // Parse user error.
    let errorMsg = idlErrors.get(errorCode);
    if (errorMsg !== undefined) {
      return new ProgramError(errorCode, errorMsg, err.logs);
    }

    // Parse framework internal error.
    errorMsg = LangErrorMessage.get(errorCode);
    if (errorMsg !== undefined) {
      return new ProgramError(errorCode, errorMsg, err.logs);
    }

    // Unable to parse the error. Just return the untranslated error.
    return null;
  }

  get program(): PublicKey | undefined {
    return this._programErrorStack?.stack[
      this._programErrorStack.stack.length - 1
    ];
  }

  get programErrorStack(): PublicKey[] | undefined {
    return this._programErrorStack?.stack;
  }

  public toString(): string {
    return this.msg;
  }
}

export function translateError(err: any, idlErrors: Map<number, string>) {
  if (features.isSet("debug-logs")) {
    console.log("Translating error:", err);
  }

  const anchorError = AnchorError.parse(err.logs);
  if (anchorError) {
    return anchorError;
  }

  const programError = ProgramError.parse(err, idlErrors);
  if (programError) {
    return programError;
  }
  if (err.logs) {
    const handler = {
      get: function (target, prop) {
        if (prop === "programErrorStack") {
          return target.programErrorStack.stack;
        } else if (prop === "program") {
          return target.programErrorStack.stack[
            err.programErrorStack.stack.length - 1
          ];
        } else {
          // this is the normal way to return all other props
          // without modifying them.
          // @ts-expect-error
          return Reflect.get(...arguments);
        }
      },
    };
    err.programErrorStack = ProgramErrorStack.parse(err.logs);
    return new Proxy(err, handler);
  }
  return err;
}

export const LangErrorCode = {
  // Instructions.
  InstructionMissing: 100,
  InstructionFallbackNotFound: 101,
  InstructionDidNotDeserialize: 102,
  InstructionDidNotSerialize: 103,

  // IDL instructions.
  IdlInstructionStub: 1000,
  IdlInstructionInvalidProgram: 1001,

  // Constraints.
  ConstraintMut: 2000,
  ConstraintHasOne: 2001,
  ConstraintSigner: 2002,
  ConstraintRaw: 2003,
  ConstraintOwner: 2004,
  ConstraintRentExempt: 2005,
  ConstraintSeeds: 2006,
  ConstraintExecutable: 2007,
  ConstraintState: 2008,
  ConstraintAssociated: 2009,
  ConstraintAssociatedInit: 2010,
  ConstraintClose: 2011,
  ConstraintAddress: 2012,
  ConstraintZero: 2013,
  ConstraintTokenMint: 2014,
  ConstraintTokenOwner: 2015,
  ConstraintMintMintAuthority: 2016,
  ConstraintMintFreezeAuthority: 2017,
  ConstraintMintDecimals: 2018,
  ConstraintSpace: 2019,

  // Require.
  RequireViolated: 2500,
  RequireEqViolated: 2501,
  RequireKeysEqViolated: 2502,
  RequireNeqViolated: 2503,
  RequireKeysNeqViolated: 2504,
  RequireGtViolated: 2505,
  RequireGteViolated: 2506,

  // Accounts.
  AccountDiscriminatorAlreadySet: 3000,
  AccountDiscriminatorNotFound: 3001,
  AccountDiscriminatorMismatch: 3002,
  AccountDidNotDeserialize: 3003,
  AccountDidNotSerialize: 3004,
  AccountNotEnoughKeys: 3005,
  AccountNotMutable: 3006,
  AccountOwnedByWrongProgram: 3007,
  InvalidProgramId: 3008,
  InvalidProgramExecutable: 3009,
  AccountNotSigner: 3010,
  AccountNotSystemOwned: 3011,
  AccountNotInitialized: 3012,
  AccountNotProgramData: 3013,
  AccountNotAssociatedTokenAccount: 3014,
  AccountSysvarMismatch: 3015,
  AccountReallocExceedsLimit: 3016,
  AccountDuplicateReallocs: 3017,

  // State.
  StateInvalidAddress: 4000,

  // Miscellaneous
  DeclaredProgramIdMismatch: 4100,

  // Used for APIs that shouldn't be used anymore.
  Deprecated: 5000,
};

export const LangErrorMessage = new Map([
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
  [
    LangErrorCode.ConstraintRentExempt,
    "A rent exemption constraint was violated",
  ],
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
  [LangErrorCode.ConstraintZero, "Expected zero account discriminant"],
  [LangErrorCode.ConstraintTokenMint, "A token mint constraint was violated"],
  [LangErrorCode.ConstraintTokenOwner, "A token owner constraint was violated"],
  [
    LangErrorCode.ConstraintMintMintAuthority,
    "A mint mint authority constraint was violated",
  ],
  [
    LangErrorCode.ConstraintMintFreezeAuthority,
    "A mint freeze authority constraint was violated",
  ],
  [
    LangErrorCode.ConstraintMintDecimals,
    "A mint decimals constraint was violated",
  ],
  [LangErrorCode.ConstraintSpace, "A space constraint was violated"],

  // Require.
  [LangErrorCode.RequireViolated, "A require expression was violated"],
  [LangErrorCode.RequireEqViolated, "A require_eq expression was violated"],
  [
    LangErrorCode.RequireKeysEqViolated,
    "A require_keys_eq expression was violated",
  ],
  [LangErrorCode.RequireNeqViolated, "A require_neq expression was violated"],
  [
    LangErrorCode.RequireKeysNeqViolated,
    "A require_keys_neq expression was violated",
  ],
  [LangErrorCode.RequireGtViolated, "A require_gt expression was violated"],
  [LangErrorCode.RequireGteViolated, "A require_gte expression was violated"],

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
    LangErrorCode.AccountOwnedByWrongProgram,
    "The given account is owned by a different program than expected",
  ],
  [LangErrorCode.InvalidProgramId, "Program ID was not as expected"],
  [LangErrorCode.InvalidProgramExecutable, "Program account is not executable"],
  [LangErrorCode.AccountNotSigner, "The given account did not sign"],
  [
    LangErrorCode.AccountNotSystemOwned,
    "The given account is not owned by the system program",
  ],
  [
    LangErrorCode.AccountNotInitialized,
    "The program expected this account to be already initialized",
  ],
  [
    LangErrorCode.AccountNotProgramData,
    "The given account is not a program data account",
  ],
  [
    LangErrorCode.AccountNotAssociatedTokenAccount,
    "The given account is not the associated token account",
  ],
  [
    LangErrorCode.AccountSysvarMismatch,
    "The given public key does not match the required sysvar",
  ],
  [
    LangErrorCode.AccountReallocExceedsLimit,
    "The account reallocation exceeds the MAX_PERMITTED_DATA_INCREASE limit",
  ],
  [
    LangErrorCode.AccountDuplicateReallocs,
    "The account was duplicated for more than one reallocation",
  ],

  // State.
  [
    LangErrorCode.StateInvalidAddress,
    "The given state account does not have the correct address",
  ],

  // Miscellaneous
  [
    LangErrorCode.DeclaredProgramIdMismatch,
    "The declared program id does not match the actual program id",
  ],

  // Deprecated
  [
    LangErrorCode.Deprecated,
    "The API being used is deprecated and should no longer be used",
  ],
]);
