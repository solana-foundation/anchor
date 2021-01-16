import camelCase from "camelcase";
import {
  Account,
  PublicKey,
  ConfirmOptions,
  Transaction,
  TransactionSignature,
  TransactionInstruction,
} from "@solana/web3.js";
import { sha256 } from "crypto-hash";
import { Idl, IdlInstruction } from "./idl";
import { IdlError, ProgramError } from "./error";
import Coder from "./coder";
import { getProvider } from "./";

/**
 * Rpcs is a dynamically generated object with rpc methods attached.
 */
export interface Rpcs {
  [key: string]: RpcFn;
}

/**
 * Ixs is a dynamically generated object with ix functions attached.
 */
export interface Ixs {
  [key: string]: IxFn;
}

/**
 * Accounts is a dynamically generated object to fetch any given account
 * of a program.
 */
export interface Accounts {
  [key: string]: AccountFn;
}

/**
 * RpcFn is a single rpc method generated from an IDL.
 */
export type RpcFn = (...args: any[]) => Promise<TransactionSignature>;

/**
 * Ix is a function to create a `TransactionInstruction` generated from an IDL.
 */
export type IxFn = (...args: any[]) => TransactionInstruction;

/**
 * Account is a function returning a deserialized account, given an address.
 */
export type AccountFn<T = any> = (address: PublicKey) => T;

/**
 * Options for an RPC invocation.
 */
export type RpcOptions = ConfirmOptions;

/**
 * RpcContext provides all arguments for an RPC/IX invocation that are not
 * covered by the instruction enum.
 */
type RpcContext = {
  // Accounts the instruction will use.
  accounts?: RpcAccounts;
  // Instructions to run *before* the specified rpc instruction.
  instructions?: TransactionInstruction[];
  // Accounts that must sign the transaction.
  signers?: Array<Account>;
  // RpcOptions.
  options?: RpcOptions;
};

/**
 * Dynamic object representing a set of accounts given to an rpc/ix invocation.
 * The name of each key should match the name for that account in the IDL.
 */
type RpcAccounts = {
  [key: string]: PublicKey;
};

/**
 * RpcFactory builds an Rpcs object for a given IDL.
 */
export class RpcFactory {
  /**
   * build dynamically generates RPC methods.
   *
   * @returns an object with all the RPC methods attached.
   */
  public static build(
    idl: Idl,
    coder: Coder,
    programId: PublicKey
  ): [Rpcs, Ixs, Accounts] {
    const rpcs: Rpcs = {};
    const ixFns: Ixs = {};
    const accountFns: Accounts = {};
    const idlErrors = parseIdlErrors(idl);
    idl.instructions.forEach((idlIx) => {
      // Function to create a raw `TransactionInstruction`.
      const ix = RpcFactory.buildIx(idlIx, coder, programId);
      // Function to invoke an RPC against a cluster.
      const rpc = RpcFactory.buildRpc(idlIx, ix, idlErrors);

      const name = camelCase(idlIx.name);
      rpcs[name] = rpc;
      ixFns[name] = ix;
    });

    if (idl.accounts) {
      idl.accounts.forEach((idlAccount) => {
        const accountFn = async (address: PublicKey): Promise<any> => {
          const provider = getProvider();
          if (provider === null) {
            throw new Error("Provider not set");
          }
          const accountInfo = await provider.connection.getAccountInfo(address);
          if (accountInfo === null) {
            throw new Error(`Entity does not exist ${address}`);
          }

          // Assert the account discriminator is correct.
          const expectedDiscriminator = Buffer.from(
            (
              await sha256(`account:${idlAccount.name}`, {
                outputFormat: "buffer",
              })
            ).slice(0, 8)
          );
          const discriminator = accountInfo.data.slice(0, 8);

          if (expectedDiscriminator.compare(discriminator)) {
            throw new Error("Invalid account discriminator");
          }

          // Chop off the discriminator before decoding.
          const data = accountInfo.data.slice(8);
          return coder.accounts.decode(idlAccount.name, data);
        };
        const name = camelCase(idlAccount.name);
        accountFns[name] = accountFn;
      });
    }

    return [rpcs, ixFns, accountFns];
  }

  private static buildIx(
    idlIx: IdlInstruction,
    coder: Coder,
    programId: PublicKey
  ): IxFn {
    if (idlIx.name === "_inner") {
      throw new IdlError("the _inner name is reserved");
    }

    const ix = (...args: any[]): TransactionInstruction => {
      const [ixArgs, ctx] = splitArgsAndCtx(idlIx, [...args]);
      validateAccounts(idlIx, ctx.accounts);
      validateInstruction(idlIx, ...args);

      const keys = idlIx.accounts.map((acc) => {
        return {
          pubkey: ctx.accounts[acc.name],
          isWritable: acc.isMut,
          isSigner: acc.isSigner,
        };
      });
      return new TransactionInstruction({
        keys,
        programId,
        data: coder.instruction.encode(toInstruction(idlIx, ...ixArgs)),
      });
    };

    return ix;
  }

  private static buildRpc(
    idlIx: IdlInstruction,
    ixFn: IxFn,
    idlErrors: Map<number, string>
  ): RpcFn {
    const rpc = async (...args: any[]): Promise<TransactionSignature> => {
      const [_, ctx] = splitArgsAndCtx(idlIx, [...args]);
      const tx = new Transaction();
      if (ctx.instructions !== undefined) {
        tx.add(...ctx.instructions);
      }
      tx.add(ixFn(...args));
      const provider = getProvider();
      if (provider === null) {
        throw new Error("Provider not found");
      }
      try {
        const txSig = await provider.send(tx, ctx.signers, ctx.options);
        return txSig;
      } catch (err) {
        let translatedErr = translateError(idlErrors, err);
        if (err === null) {
          throw err;
        }
        throw translatedErr;
      }
    };

    return rpc;
  }
}

function translateError(
  idlErrors: Map<number, string>,
  err: any
): Error | null {
  // TODO: don't rely on the error string. web3.js should preserve the error
  //       code information instead of giving us an untyped string.
  let components = err.toString().split("custom program error: ");
  if (components.length === 2) {
    try {
      const errorCode = parseInt(components[1]);
      let errorMsg = idlErrors.get(errorCode);
      if (errorMsg === undefined) {
        // Unexpected error code so just throw the untranslated error.
        throw err;
      }
      return new ProgramError(errorCode, errorMsg);
    } catch (parseErr) {
      // Unable to parse the error. Just return the untranslated error.
      return null;
    }
  }
}

function parseIdlErrors(idl: Idl): Map<number, string> {
  const errors = new Map();
  if (idl.errors) {
    idl.errors.forEach((e) => {
      let msg = e.msg ?? e.name;
      errors.set(e.code, msg);
    });
  }
  return errors;
}

function splitArgsAndCtx(
  idlIx: IdlInstruction,
  args: any[]
): [any[], RpcContext] {
  let options = {};

  const inputLen = idlIx.args ? idlIx.args.length : 0;
  if (args.length > inputLen) {
    if (args.length !== inputLen + 1) {
      throw new Error("provided too many arguments ${args}");
    }
    options = args.pop();
  }

  return [args, options];
}

function toInstruction(idlIx: IdlInstruction, ...args: any[]) {
  if (idlIx.args.length != args.length) {
    throw new Error("Invalid argument length");
  }
  const ix: { [key: string]: any } = {};
  let idx = 0;
  idlIx.args.forEach((ixArg) => {
    ix[ixArg.name] = args[idx];
    idx += 1;
  });

  // JavaScript representation of the rust enum variant.
  const name = camelCase(idlIx.name);
  const ixVariant: { [key: string]: any } = {};
  ixVariant[name] = ix;

  return ixVariant;
}

// Throws error if any account required for the `ix` is not given.
function validateAccounts(ix: IdlInstruction, accounts: RpcAccounts) {
  ix.accounts.forEach((acc) => {
    if (accounts[acc.name] === undefined) {
      throw new Error(`Invalid arguments: ${acc.name} not provided.`);
    }
  });
}

// Throws error if any argument required for the `ix` is not given.
function validateInstruction(ix: IdlInstruction, ...args: any[]) {
  // todo
}
