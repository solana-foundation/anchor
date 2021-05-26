import { Transaction } from "@solana/web3.js";
import { IdlInstruction } from "../../idl";
import { splitArgsAndCtx } from "../context";
import { InstructionFn } from "./instruction";

export default class TransactionFactory {
  public static build(
    idlIx: IdlInstruction,
    ixFn: InstructionFn
  ): TransactionFn {
    const txFn = (...args: any[]): Transaction => {
      const [, ctx] = splitArgsAndCtx(idlIx, [...args]);
      const tx = new Transaction();
      if (ctx.instructions !== undefined) {
        tx.add(...ctx.instructions);
      }
      tx.add(ixFn(...args));
      return tx;
    };

    return txFn;
  }
}

/**
 * The namespace provides functions to build [[Transaction]] objects for each
 * method of a program.
 *
 * ## Usage
 *
 * ```javascript
 * program.transaction.<method>(...args, ctx);
 * ```
 *
 * ## Parameters
 *
 * 1. `args` - The positional arguments for the program. The type and number
 *    of these arguments depend on the program being used.
 * 2. `ctx`  - [[Context]] non-argument parameters to pass to the method.
 *    Always the last parameter in the method call.
 *
 * ## Example
 *
 * To create an instruction for the `increment` method above,
 *
 * ```javascript
 * const tx = await program.transaction.increment({
 *   accounts: {
 *     counter,
 *   },
 * });
 * ```
 */
export interface TransactionNamespace {
  [key: string]: TransactionFn;
}

/**
 * Tx is a function to create a `Transaction` for a given program instruction.
 */
export type TransactionFn = (...args: any[]) => Transaction;
