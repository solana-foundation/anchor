import camelCase from "camelcase";
import {
  Account,
  PublicKey,
  ConfirmOptions,
  Transaction,
  TransactionSignature,
  TransactionInstruction,
} from "@solana/web3.js";
import { Idl, IdlInstruction } from "./idl";
import { IdlError } from "./error";
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
 * RpcFn is a single rpc method.
 */
export type RpcFn = (...args: any[]) => Promise<any>;

/**
 * Ix is a function to create a `TransactionInstruction`.
 */
export type IxFn = (...args: any[]) => TransactionInstruction;

/**
 * Account is a function returning a deserialized account, given an address.
 */
export type AccountFn = (address: PublicKey) => any;

/**
 * Options for an RPC invocation.
 */
type RpcOptions = ConfirmOptions;

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
    idl.instructions.forEach((idlIx) => {
      // Function to create a raw `TransactionInstruction`.
      const ix = RpcFactory.buildIx(idlIx, coder, programId);
      // Function to invoke an RPC against a cluster.
      const rpc = RpcFactory.buildRpc(idlIx, ix);

      const name = camelCase(idlIx.name);
      rpcs[name] = rpc;
      ixFns[name] = ix;
    });

		if (idl.accounts) {
			idl.accounts.forEach((idlAccount) => {
				// todo
				const accountFn = async (address: PublicKey): Promise<any> => {
					const provider = getProvider();
					if (provider === null) {
						throw new Error("Provider not set");
					}
					const accountInfo = await provider.connection.getAccountInfo(address);
					if (accountInfo === null) {
						throw new Error(`Entity does not exist ${address}`);
					}
					return coder.accounts.decode(idlAccount.name, accountInfo.data);
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

  private static buildRpc(idlIx: IdlInstruction, ixFn: IxFn): RpcFn {
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

      const txSig = await provider.send(tx, ctx.signers, ctx.options);
      return txSig;
    };

    return rpc;
  }
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
