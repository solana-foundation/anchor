import { TransactionSignature } from "@solana/web3.js";
import Provider from "../../provider";
import { IdlInstruction } from "../../idl";
import { Context, splitArgsAndCtx } from "../context";
import { TransactionFn } from "./transaction";
import { ProgramError } from "../../error";

export default class RpcFactory {
  public static build(
    idlIx: IdlInstruction,
    txFn: TransactionFn,
    idlErrors: Map<number, string>,
    provider: Provider
  ): RpcFn {
    const rpc = async (...args: any[]): Promise<TransactionSignature> => {
      const tx = txFn(...args);
      const [, ctx] = splitArgsAndCtx(idlIx, [...args]);
      try {
        const txSig = await provider.send(tx, ctx.signers, ctx.options);
        return txSig;
      } catch (err) {
        console.log("Translating error", err);
        let translatedErr = ProgramError.parse(err, idlErrors);
        if (translatedErr === null) {
          throw err;
        }
        throw translatedErr;
      }
    };

    return rpc;
  }
}

/**
 * The namespace provides async methods to send signed transactions for each
 * *non*-state method on Anchor program.
 *
 * Keys are method names, values are RPC functions returning a
 * [[TransactionInstruction]].
 *
 * ## Usage
 *
 * ```javascript
 * rpc.<method>(...args, ctx);
 * ```
 *
 * ## Parameters
 *
 * 1. `args` - The positional arguments for the program. The type and number
 *    of these arguments depend on the program being used.
 * 2. `ctx`  - [[Context]] non-argument parameters to pass to the method.
 *    Always the last parameter in the method call.
 * ```
 *
 * ## Example
 *
 * To send a transaction invoking the `increment` method above,
 *
 * ```javascript
 * const txSignature = await program.rpc.increment({
 *   accounts: {
 *     counter,
 *     authority,
 *   },
 * });
 * ```
 */
export interface RpcNamespace {
  [key: string]: RpcFn;
}

/**
 * RpcFn is a single RPC method generated from an IDL, sending a transaction
 * paid for and signed by the configured provider.
 */
export type RpcFn = (...args: any[]) => Promise<TransactionSignature>;
