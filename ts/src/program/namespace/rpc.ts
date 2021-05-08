import { TransactionSignature } from "@solana/web3.js";
import Provider from "../../provider";
import { IdlInstruction } from "../../idl";
import { translateError } from "../common";
import { splitArgsAndCtx } from "../context";
import { TxFn } from "./transaction";

/**
 * Dynamically generated rpc namespace.
 */
export interface Rpcs {
  [key: string]: RpcFn;
}

/**
 * RpcFn is a single rpc method generated from an IDL.
 */
export type RpcFn = (...args: any[]) => Promise<TransactionSignature>;

export default class RpcNamespace {
  // Builds the rpc namespace.
  public static build(
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
}
