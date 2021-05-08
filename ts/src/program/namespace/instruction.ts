import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { IdlAccount, IdlInstruction, IdlAccountItem } from "../../idl";
import { IdlError } from "../../error";
import Coder from "../../coder";
import { toInstruction, validateAccounts } from "../common";
import { RpcAccounts, splitArgsAndCtx } from "../context";

/**
 * Dynamically generated instruction namespace.
 */
export interface Ixs {
  [key: string]: IxFn;
}

/**
 * Ix is a function to create a `TransactionInstruction` generated from an IDL.
 */
export type IxFn = IxProps & ((...args: any[]) => any);
type IxProps = {
  accounts: (ctx: RpcAccounts) => any;
};

export default class InstructionNamespace {
  // Builds the instuction namespace.
  public static build(
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

      const keys = InstructionNamespace.accountsArray(
        ctx.accounts,
        idlIx.accounts
      );

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
      return InstructionNamespace.accountsArray(accs, idlIx.accounts);
    };

    return ix;
  }

  public static accountsArray(
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
          return InstructionNamespace.accountsArray(
            rpcAccs,
            nestedAccounts
          ).flat();
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
}

// Throws error if any argument required for the `ix` is not given.
function validateInstruction(ix: IdlInstruction, ...args: any[]) {
  // todo
}
