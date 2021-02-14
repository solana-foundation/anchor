import camelCase from "camelcase";
import EventEmitter from "eventemitter3";
import * as bs58 from "bs58";
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
  Commitment,
} from "@solana/web3.js";
import Provider from "./provider";
import {
  Idl,
  IdlAccount,
  IdlInstruction,
  IdlAccountItem,
  IdlStateMethod,
} from "./idl";
import { IdlError, ProgramError } from "./error";
import Coder, {
  ACCOUNT_DISCRIMINATOR_SIZE,
  accountDiscriminator,
  stateDiscriminator,
  accountSize,
} from "./coder";

/**
 * Dynamically generated rpc namespace.
 */
export interface Rpcs {
  [key: string]: RpcFn;
}

/**
 * Dynamically generated instruction namespace.
 */
export interface Ixs {
  [key: string]: IxFn;
}

/**
 * Dynamically generated transaction namespace.
 */
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
export type IxFn = IxProps & ((...args: any[]) => any);
type IxProps = {
  accounts: (ctx: RpcAccounts) => any;
};

/**
 * Tx is a function to create a `Transaction` generate from an IDL.
 */
export type TxFn = (...args: any[]) => Transaction;

/**
 * Account is a function returning a deserialized account, given an address.
 */
export type AccountFn<T = any> = AccountProps & ((address: PublicKey) => T);

/**
 * Deserialized account owned by a program.
 */
export type ProgramAccount<T = any> = {
  publicKey: PublicKey;
  account: T;
};

/**
 * Non function properties on the acccount namespace.
 */
type AccountProps = {
  size: number;
  all: (filter?: Buffer) => Promise<ProgramAccount<any>[]>;
  subscribe: (address: PublicKey, commitment?: Commitment) => EventEmitter;
  unsubscribe: (address: PublicKey) => void;
  createInstruction: (account: Account) => Promise<TransactionInstruction>;
};

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

// Tracks all subscriptions.
const subscriptions: Map<string, Subscription> = new Map();

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
    programId: PublicKey,
    provider: Provider
  ): [Rpcs, Ixs, Txs, Accounts, State] {
    const idlErrors = parseIdlErrors(idl);

    const rpcs: Rpcs = {};
    const ixFns: Ixs = {};
    const txFns: Txs = {};
    const state = RpcFactory.buildState(
      idl,
      coder,
      programId,
      idlErrors,
      provider
    );

    idl.instructions.forEach((idlIx) => {
      const name = camelCase(idlIx.name);
      // Function to create a raw `TransactionInstruction`.
      const ix = RpcFactory.buildIx(idlIx, coder, programId);
      // Ffnction to create a `Transaction`.
      const tx = RpcFactory.buildTx(idlIx, ix);
      // Function to invoke an RPC against a cluster.
      const rpc = RpcFactory.buildRpc(idlIx, tx, idlErrors, provider);
      rpcs[name] = rpc;
      ixFns[name] = ix;
      txFns[name] = tx;
    });

    const accountFns = idl.accounts
      ? RpcFactory.buildAccounts(idl, coder, programId, provider)
      : {};

    return [rpcs, ixFns, txFns, accountFns, state];
  }

  // Builds the state namespace.
  private static buildState(
    idl: Idl,
    coder: Coder,
    programId: PublicKey,
    idlErrors: Map<number, string>,
    provider: Provider
  ): State | undefined {
    if (idl.state === undefined) {
      return undefined;
    }

    // Fetches the state object from the blockchain.
    const state = async (): Promise<any> => {
      const addr = await programStateAddress(programId);
      const accountInfo = await provider.connection.getAccountInfo(addr);
      if (accountInfo === null) {
        throw new Error(`Account does not exist ${addr.toString()}`);
      }
      // Assert the account discriminator is correct.
      const expectedDiscriminator = await stateDiscriminator(
        idl.state.struct.name
      );
      if (expectedDiscriminator.compare(accountInfo.data.slice(0, 8))) {
        throw new Error("Invalid account discriminator");
      }
      return coder.state.decode(accountInfo.data);
    };

    // Namespace with all rpc functions.
    const rpc: Rpcs = {};
    const ix: Ixs = {};

    idl.state.methods.forEach((m: IdlStateMethod) => {
      const accounts = async (accounts: RpcAccounts): Promise<any> => {
        const keys = await stateInstructionKeys(
          programId,
          provider,
          m,
          accounts
        );
        return keys.concat(RpcFactory.accountsArray(accounts, m.accounts));
      };
      const ixFn = async (...args: any[]): Promise<TransactionInstruction> => {
        const [ixArgs, ctx] = splitArgsAndCtx(m, [...args]);
        return new TransactionInstruction({
          keys: await accounts(ctx.accounts),
          programId,
          data: coder.instruction.encodeState(
            m.name,
            toInstruction(m, ...ixArgs)
          ),
        });
      };
      ixFn["accounts"] = accounts;
      ix[m.name] = ixFn;

      rpc[m.name] = async (...args: any[]): Promise<TransactionSignature> => {
        const [_, ctx] = splitArgsAndCtx(m, [...args]);
        const tx = new Transaction();
        if (ctx.instructions !== undefined) {
          tx.add(...ctx.instructions);
        }
        tx.add(await ix[m.name](...args));
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
    });

    state["rpc"] = rpc;
    state["instruction"] = ix;
    // Calculates the address of the program's global state object account.
    state["address"] = async (): Promise<PublicKey> =>
      programStateAddress(programId);

    // Subscription singleton.
    let sub: null | Subscription = null;

    // Subscribe to account changes.
    state["subscribe"] = (commitment?: Commitment): EventEmitter => {
      if (sub !== null) {
        return sub.ee;
      }
      const ee = new EventEmitter();

      state["address"]().then((address) => {
        const listener = provider.connection.onAccountChange(
          address,
          (acc) => {
            const account = coder.state.decode(acc.data);
            ee.emit("change", account);
          },
          commitment
        );

        sub = {
          ee,
          listener,
        };
      });

      return ee;
    };

    // Unsubscribe from account changes.
    state["unsubscribe"] = () => {
      if (sub !== null) {
        provider.connection
          .removeAccountChangeListener(sub.listener)
          .then(async () => {
            sub = null;
          })
          .catch(console.error);
      }
    };

    return state;
  }

  // Builds the instuction namespace.
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
        console.log("Outgoing account metas:", keys);
      }
      return new TransactionInstruction({
        keys,
        programId,
        data: coder.instruction.encode(
          idlIx.name,
          toInstruction(idlIx, ...ixArgs)
        ),
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

  // Builds the rpc namespace.
  private static buildRpc(
    idlIx: IdlInstruction,
    txFn: TxFn,
    idlErrors: Map<number, string>,
    provider: Provider
  ): RpcFn {
    const rpc = async (...args: any[]): Promise<TransactionSignature> => {
      const tx = txFn(...args);
      const [_, ctx] = splitArgsAndCtx(idlIx, [...args]);
      try {
        const txSig = await provider.send(tx, ctx.signers, ctx.options);
        return txSig;
      } catch (err) {
        console.log("Translating error", err);
        let translatedErr = translateError(idlErrors, err);
        if (translatedErr === null) {
          throw err;
        }
        throw translatedErr;
      }
    };

    return rpc;
  }

  // Builds the transaction namespace.
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

  // Returns the generated accounts namespace.
  private static buildAccounts(
    idl: Idl,
    coder: Coder,
    programId: PublicKey,
    provider: Provider
  ): Accounts {
    const accountFns: Accounts = {};

    idl.accounts.forEach((idlAccount) => {
      const name = camelCase(idlAccount.name);

      // Fetches the decoded account from the network.
      const accountsNamespace = async (address: PublicKey): Promise<any> => {
        const accountInfo = await provider.connection.getAccountInfo(address);
        if (accountInfo === null) {
          throw new Error(`Account does not exist ${address.toString()}`);
        }

        // Assert the account discriminator is correct.
        const discriminator = await accountDiscriminator(idlAccount.name);
        if (discriminator.compare(accountInfo.data.slice(0, 8))) {
          throw new Error("Invalid account discriminator");
        }

        return coder.accounts.decode(idlAccount.name, accountInfo.data);
      };

      // Returns the size of the account.
      // @ts-ignore
      accountsNamespace["size"] =
        ACCOUNT_DISCRIMINATOR_SIZE + accountSize(idl, idlAccount);

      // Returns an instruction for creating this account.
      // @ts-ignore
      accountsNamespace["createInstruction"] = async (
        account: Account,
        sizeOverride?: number
      ): Promise<TransactionInstruction> => {
        // @ts-ignore
        const size = accountsNamespace["size"];

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

      // Subscribes to all changes to this account.
      // @ts-ignore
      accountsNamespace["subscribe"] = (
        address: PublicKey,
        commitment?: Commitment
      ): EventEmitter => {
        if (subscriptions.get(address.toString())) {
          return subscriptions.get(address.toString()).ee;
        }
        const ee = new EventEmitter();

        const listener = provider.connection.onAccountChange(
          address,
          (acc) => {
            const account = coder.accounts.decode(idlAccount.name, acc.data);
            ee.emit("change", account);
          },
          commitment
        );

        subscriptions.set(address.toString(), {
          ee,
          listener,
        });

        return ee;
      };

      // Unsubscribes to account changes.
      // @ts-ignore
      accountsNamespace["unsubscribe"] = (address: PublicKey) => {
        let sub = subscriptions.get(address.toString());
        if (subscriptions) {
          provider.connection
            .removeAccountChangeListener(sub.listener)
            .then(() => {
              subscriptions.delete(address.toString());
            })
            .catch(console.error);
        }
      };

      // Returns all instances of this account type for the program.
      // @ts-ignore
      accountsNamespace["all"] = async (
        filter?: Buffer
      ): Promise<ProgramAccount<any>[]> => {
        let bytes = await accountDiscriminator(idlAccount.name);
        if (filter !== undefined) {
          bytes = Buffer.concat([bytes, filter]);
        }
        // @ts-ignore
        let resp = await provider.connection._rpcRequest("getProgramAccounts", [
          programId.toBase58(),
          {
            commitment: provider.connection.commitment,
            filters: [
              {
                memcmp: {
                  offset: 0,
                  bytes: bs58.encode(bytes),
                },
              },
            ],
          },
        ]);
        if (resp.error) {
          console.error(resp);
          throw new Error("Failed to get accounts");
        }
        return (
          resp.result
            // @ts-ignore
            .map(({ pubkey, account: { data } }) => {
              data = bs58.decode(data);
              return {
                publicKey: new PublicKey(pubkey),
                account: coder.accounts.decode(idlAccount.name, data),
              };
            })
        );
      };

      accountFns[name] = accountsNamespace;
    });

    return accountFns;
  }
}

type Subscription = {
  listener: number;
  ee: EventEmitter;
};

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

  return ix;
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

// Calculates the deterministic address of the program's "state" account.
async function programStateAddress(programId: PublicKey): Promise<PublicKey> {
  let [registrySigner, _nonce] = await PublicKey.findProgramAddress(
    [],
    programId
  );
  return PublicKey.createWithSeed(registrySigner, "unversioned", programId);
}

// Returns the common keys that are prepended to all instructions targeting
// the "state" of a program.
async function stateInstructionKeys(
  programId: PublicKey,
  provider: Provider,
  m: IdlStateMethod,
  accounts: RpcAccounts
) {
  if (m.name === "new") {
    // Ctor `new` method.
    const [programSigner, _nonce] = await PublicKey.findProgramAddress(
      [],
      programId
    );
    return [
      {
        pubkey: provider.wallet.publicKey,
        isWritable: false,
        isSigner: true,
      },
      {
        pubkey: await programStateAddress(programId),
        isWritable: true,
        isSigner: false,
      },
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
    ];
  } else {
    validateAccounts(m.accounts, accounts);
    return [
      {
        pubkey: await programStateAddress(programId),
        isWritable: true,
        isSigner: false,
      },
    ];
  }
}
