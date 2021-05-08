import {
  Account,
  AccountMeta,
  PublicKey,
  ConfirmOptions,
  TransactionInstruction,
} from "@solana/web3.js";
import { IdlInstruction } from "../idl";

/**
 * RpcContext provides all arguments for an RPC/IX invocation that are not
 * covered by the instruction enum.
 */
type RpcContext = {
  // Accounts the instruction will use.
  accounts?: RpcAccounts;
  // All accounts to pass into an instruction *after* the main `accounts`.
  remainingAccounts?: AccountMeta[];
  // Accounts that must sign the transaction.
  signers?: Array<Account>;
  // Instructions to run *before* the specified rpc instruction.
  instructions?: TransactionInstruction[];
  // RpcOptions.
  options?: RpcOptions;
  // Private namespace for dev.
  __private?: { logAccounts: boolean };
};

/**
 * Dynamic object representing a set of accounts given to an rpc/ix invocation.
 * The name of each key should match the name for that account in the IDL.
 */
export type RpcAccounts = {
  [key: string]: PublicKey | RpcAccounts;
};

/**
 * Options for an RPC invocation.
 */
export type RpcOptions = ConfirmOptions;

export function splitArgsAndCtx(
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
