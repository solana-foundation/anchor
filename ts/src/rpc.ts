import camelCase from "camelcase";
import {
  Account,
  AccountMeta,
  PublicKey,
  ConfirmOptions,
  SystemProgram,
  Transaction,
  TransactionSignature,
  TransactionInstruction,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import { sha256 } from "crypto-hash";
import {
  Idl,
  IdlAccount,
  IdlInstruction,
  IdlTypeDef,
  IdlType,
  IdlField,
  IdlEnumVariant,
  IdlAccountItem,
  IdlStateMethod,
} from "./idl";
import { IdlError, ProgramError } from "./error";
import Coder from "./coder";
import { getProvider } from "./";

/**
 * Number of bytes of the account discriminator.
 */
const ACCOUNT_DISCRIMINATOR_SIZE = 8;

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

export interface Txs {
  [key: string]: TxFn;
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
 * Tx is a function to create a `Transaction` generate from an IDL.
 */
export type TxFn = (...args: any[]) => Transaction;

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
  remainingAccounts?: AccountMeta[];
  // Instructions to run *before* the specified rpc instruction.
  instructions?: TransactionInstruction[];
  // Accounts that must sign the transaction.
  signers?: Array<Account>;
  // RpcOptions.
  options?: RpcOptions;
  __private?: { logAccounts: boolean };
};

/**
 * Dynamic object representing a set of accounts given to an rpc/ix invocation.
 * The name of each key should match the name for that account in the IDL.
 */
type RpcAccounts = {
  [key: string]: PublicKey | RpcAccounts;
};

export type State = {
  address: () => Promise<PublicKey>;
  rpc: Rpcs;
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
  ): [Rpcs, Ixs, Txs, Accounts, State] {
    const idlErrors = parseIdlErrors(idl);

    const rpcs: Rpcs = {};
    const ixFns: Ixs = {};
    const txFns: Txs = {};
    const state = RpcFactory.buildState(idl, coder, programId, idlErrors);

    idl.instructions.forEach((idlIx) => {
      // Function to create a raw `TransactionInstruction`.
      const ix = RpcFactory.buildIx(idlIx, coder, programId);
      // Ffnction to create a `Transaction`.
      const tx = RpcFactory.buildTx(idlIx, ix);
      // Function to invoke an RPC against a cluster.
      const rpc = RpcFactory.buildRpc(idlIx, tx, idlErrors);

      const name = camelCase(idlIx.name);
      rpcs[name] = rpc;
      ixFns[name] = ix;
      txFns[name] = tx;
    });

    const accountFns = idl.accounts
      ? RpcFactory.buildAccounts(idl, coder, programId)
      : {};

    return [rpcs, ixFns, txFns, accountFns, state];
  }

  private static buildState(
    idl: Idl,
    coder: Coder,
    programId: PublicKey,
    idlErrors: Map<number, string>
  ): State | undefined {
    if (idl.state === undefined) {
      return undefined;
    }
    let address = async () => {
      let [registrySigner, _nonce] = await PublicKey.findProgramAddress(
        [],
        programId
      );
      return PublicKey.createWithSeed(registrySigner, "unversioned", programId);
    };

    const rpc: Rpcs = {};
    idl.state.methods.forEach((m: IdlStateMethod) => {
      if (m.name === "new") {
        // Ctor `new` method.
        rpc[m.name] = async (...args: any[]): Promise<TransactionSignature> => {
          const [ixArgs, ctx] = splitArgsAndCtx(m, [...args]);
          const tx = new Transaction();
          const [programSigner, _nonce] = await PublicKey.findProgramAddress(
            [],
            programId
          );
          const ix = new TransactionInstruction({
            keys: [
              {
                pubkey: getProvider().wallet.publicKey,
                isWritable: false,
                isSigner: true,
              },
              { pubkey: await address(), isWritable: true, isSigner: false },
              { pubkey: programSigner, isWritable: false, isSigner: false },
              {
                pubkey: SystemProgram.programId,
                isWritable: false,
                isSigner: false,
              },

              { pubkey: programId, isWritable: false, isSigner: false },
              {
                pubkey: SYSVAR_RENT_PUBKEY,
                isWritable: false,
                isSigner: false,
              },
            ].concat(RpcFactory.accountsArray(ctx.accounts, m.accounts)),
            programId,
            data: coder.instruction.encode(toInstruction(m, ...ixArgs)),
          });

          tx.add(ix);

          const provider = getProvider();
          if (provider === null) {
            throw new Error("Provider not found");
          }
          try {
            const txSig = await provider.send(tx, ctx.signers, ctx.options);
            return txSig;
          } catch (err) {
            let translatedErr = translateError(idlErrors, err);
            if (translatedErr === null) {
              throw err;
            }
            throw translatedErr;
          }
        };
      } else {
        rpc[m.name] = async (...args: any[]): Promise<TransactionSignature> => {
          const [ixArgs, ctx] = splitArgsAndCtx(m, [...args]);
          validateAccounts(m.accounts, ctx.accounts);
          const tx = new Transaction();

          const keys = [
            { pubkey: await address(), isWritable: true, isSigner: false },
          ].concat(RpcFactory.accountsArray(ctx.accounts, m.accounts));

          tx.add(
            new TransactionInstruction({
              keys,
              programId,
              data: coder.instruction.encode(toInstruction(m, ...ixArgs)),
            })
          );

          const provider = getProvider();
          if (provider === null) {
            throw new Error("Provider not found");
          }
          try {
            const txSig = await provider.send(tx, ctx.signers, ctx.options);
            return txSig;
          } catch (err) {
            let translatedErr = translateError(idlErrors, err);
            if (translatedErr === null) {
              throw err;
            }
            throw translatedErr;
          }
        };
      }
    });

    // Fetches the state object from the blockchain.
    const state = async (): Promise<any> => {
      const addr = await address();
      const provider = getProvider();
      if (provider === null) {
        throw new Error("Provider not set");
      }
      const accountInfo = await provider.connection.getAccountInfo(addr);
      if (accountInfo === null) {
        throw new Error(`Entity does not exist ${address}`);
      }
      // Assert the account discriminator is correct.
      const expectedDiscriminator = Buffer.from(
        (
          await sha256(`state:${idl.state.struct.name}`, {
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
      return coder.state.decode(data);
    };

    state["address"] = address;
    state["rpc"] = rpc;

    return state;
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
      validateAccounts(idlIx.accounts, ctx.accounts);
      validateInstruction(idlIx, ...args);

      const keys = RpcFactory.accountsArray(ctx.accounts, idlIx.accounts);

      if (ctx.remainingAccounts !== undefined) {
        keys.push(...ctx.remainingAccounts);
      }

      if (ctx.__private && ctx.__private.logAccounts) {
        console.log("Outoing account metas:", keys);
      }
      return new TransactionInstruction({
        keys,
        programId,
        data: coder.instruction.encode(toInstruction(idlIx, ...ixArgs)),
      });
    };

    // Utility fn for ordering the accounts for this instruction.
    ix["accounts"] = (accs: RpcAccounts) => {
      return RpcFactory.accountsArray(accs, idlIx.accounts);
    };

    return ix;
  }

  private static accountsArray(
    ctx: RpcAccounts,
    accounts: IdlAccountItem[]
  ): any {
    return accounts
      .map((acc: IdlAccountItem) => {
        // Nested accounts.
        // @ts-ignore
        const nestedAccounts: IdlAccountItem[] | undefined = acc.accounts;
        if (nestedAccounts !== undefined) {
          const rpcAccs = ctx[acc.name] as RpcAccounts;
          return RpcFactory.accountsArray(rpcAccs, nestedAccounts).flat();
        } else {
          const account: IdlAccount = acc as IdlAccount;
          return {
            pubkey: ctx[acc.name],
            isWritable: account.isMut,
            isSigner: account.isSigner,
          };
        }
      })
      .flat();
  }

  private static buildRpc(
    idlIx: IdlInstruction,
    txFn: TxFn,
    idlErrors: Map<number, string>
  ): RpcFn {
    const rpc = async (...args: any[]): Promise<TransactionSignature> => {
      const tx = txFn(...args);
      const [_, ctx] = splitArgsAndCtx(idlIx, [...args]);
      const provider = getProvider();
      if (provider === null) {
        throw new Error("Provider not found");
      }
      try {
        const txSig = await provider.send(tx, ctx.signers, ctx.options);
        return txSig;
      } catch (err) {
        let translatedErr = translateError(idlErrors, err);
        if (translatedErr === null) {
          throw err;
        }
        throw translatedErr;
      }
    };

    return rpc;
  }

  private static buildTx(idlIx: IdlInstruction, ixFn: IxFn): TxFn {
    const txFn = (...args: any[]): Transaction => {
      const [_, ctx] = splitArgsAndCtx(idlIx, [...args]);
      const tx = new Transaction();
      if (ctx.instructions !== undefined) {
        tx.add(...ctx.instructions);
      }
      tx.add(ixFn(...args));
      return tx;
    };

    return txFn;
  }

  private static buildAccounts(
    idl: Idl,
    coder: Coder,
    programId: PublicKey
  ): Accounts {
    const accountFns: Accounts = {};
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
      const size = ACCOUNT_DISCRIMINATOR_SIZE + accountSize(idl, idlAccount);
      // @ts-ignore
      accountFns[name]["size"] = size;
      // @ts-ignore
      accountFns[name]["createInstruction"] = async (
        account: Account,
        sizeOverride?: number
      ): Promise<TransactionInstruction> => {
        const provider = getProvider();
        return SystemProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          newAccountPubkey: account.publicKey,
          space: sizeOverride ?? size,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(
            sizeOverride ?? size
          ),
          programId,
        });
      };
    });
    return accountFns;
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
        return null;
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

// Allow either IdLInstruction or IdlStateMethod since the types share fields.
function toInstruction(idlIx: IdlInstruction | IdlStateMethod, ...args: any[]) {
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
function validateAccounts(ixAccounts: IdlAccountItem[], accounts: RpcAccounts) {
  ixAccounts.forEach((acc) => {
    // @ts-ignore
    if (acc.accounts !== undefined) {
      // @ts-ignore
      validateAccounts(acc.accounts, accounts[acc.name]);
    } else {
      if (accounts[acc.name] === undefined) {
        throw new Error(`Invalid arguments: ${acc.name} not provided.`);
      }
    }
  });
}

// Throws error if any argument required for the `ix` is not given.
function validateInstruction(ix: IdlInstruction, ...args: any[]) {
  // todo
}

function accountSize(idl: Idl, idlAccount: IdlTypeDef): number | undefined {
  if (idlAccount.type.kind === "enum") {
    let variantSizes = idlAccount.type.variants.map(
      (variant: IdlEnumVariant) => {
        if (variant.fields === undefined) {
          return 0;
        }
        // @ts-ignore
        return (
          variant.fields
            // @ts-ignore
            .map((f: IdlField | IdlType) => {
              // @ts-ignore
              if (f.name === undefined) {
                throw new Error("Tuple enum variants not yet implemented.");
              }
              // @ts-ignore
              return typeSize(idl, f.type);
            })
            .reduce((a: number, b: number) => a + b)
        );
      }
    );
    return Math.max(...variantSizes) + 1;
  }
  if (idlAccount.type.fields === undefined) {
    return 0;
  }
  return idlAccount.type.fields
    .map((f) => typeSize(idl, f.type))
    .reduce((a, b) => a + b);
}

// Returns the size of the type in bytes. For variable length types, just return
// 1. Users should override this value in such cases.
function typeSize(idl: Idl, ty: IdlType): number {
  switch (ty) {
    case "bool":
      return 1;
    case "u8":
      return 1;
    case "i8":
      return 1;
    case "u16":
      return 2;
    case "u32":
      return 4;
    case "u64":
      return 8;
    case "i64":
      return 8;
    case "bytes":
      return 1;
    case "string":
      return 1;
    case "publicKey":
      return 32;
    default:
      // @ts-ignore
      if (ty.vec !== undefined) {
        return 1;
      }
      // @ts-ignore
      if (ty.option !== undefined) {
        // @ts-ignore
        return 1 + typeSize(ty.option);
      }
      // @ts-ignore
      if (ty.defined !== undefined) {
        // @ts-ignore
        const filtered = idl.types.filter((t) => t.name === ty.defined);
        if (filtered.length !== 1) {
          throw new IdlError(`Type not found: ${JSON.stringify(ty)}`);
        }
        let typeDef = filtered[0];

        return accountSize(idl, typeDef);
      }
      throw new Error(`Invalid type ${JSON.stringify(ty)}`);
  }
}
