import { Transaction } from "@solana/web3.js";
import { IdlInstruction } from "../../idl";
import { splitArgsAndCtx } from "../context";
import { IxFn } from "./instruction";

/**
 * Dynamically generated transaction namespace.
 */
export interface Txs {
  [key: string]: TxFn;
}

/**
 * Tx is a function to create a `Transaction` generate from an IDL.
 */
export type TxFn = (...args: any[]) => Transaction;

export default class TransactionNamespace {
  // Builds the transaction namespace.
  public static build(idlIx: IdlInstruction, ixFn: IxFn): TxFn {
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
}
