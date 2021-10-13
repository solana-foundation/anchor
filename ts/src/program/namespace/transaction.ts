import { Transaction } from "@solana/web3.js";
import { Idl, IdlInstruction } from "../../idl";
import { splitArgsAndCtx } from "../context";
import { InstructionFn } from "./instruction";
import {
  AllInstructions,
  InstructionContextFn,
  MakeInstructionsNamespace,
} from "./types";

export default class TransactionFactory {
  public static build<IDL extends Idl, I extends AllInstructions<IDL>>(
    idlIx: I,
    ixFn: InstructionFn<IDL, I>
  ): TransactionFn<IDL, I> {
    const txFn: TransactionFn<IDL, I> = (...args): Transaction => {
      const [, ctx] = splitArgsAndCtx(idlIx, [...args]);
      const tx = new Transaction();
      ctx.instructions?.forEach((ix) => tx.add(ix));
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
export type TransactionNamespace<
  IDL extends Idl = Idl,
  I extends AllInstructions<IDL> = AllInstructions<IDL>
> = MakeInstructionsNamespace<IDL, I, Transaction>;

/**
 * Tx is a function to create a `Transaction` for a given program instruction.
 */
export type TransactionFn<
  IDL extends Idl = Idl,
  I extends AllInstructions<IDL> = AllInstructions<IDL>
> = InstructionContextFn<IDL, I, Transaction>;
