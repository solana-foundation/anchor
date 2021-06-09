import EventEmitter from "eventemitter3";
import { PublicKey } from "@solana/web3.js";
import { Idl, IdlInstruction, IdlAccountItem, IdlStateMethod } from "../idl";
import { Accounts } from "./context";

export type Subscription = {
  listener: number;
  ee: EventEmitter;
};

export function parseIdlErrors(idl: Idl): Map<number, string> {
  const errors = new Map();
  if (idl.errors) {
    idl.errors.forEach((e) => {
      let msg = e.msg ?? e.name;
      errors.set(e.code, msg);
    });
  }
  return errors;
}

// Allow either IdLInstruction or IdlStateMethod since the types share fields.
export function toInstruction(
  idlIx: IdlInstruction | IdlStateMethod,
  ...args: any[]
) {
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
export function validateAccounts(
  ixAccounts: IdlAccountItem[],
  accounts: Accounts
) {
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

// Translates an address to a Pubkey.
export function translateAddress(address: Address): PublicKey {
  if (typeof address === "string") {
    const pk = new PublicKey(address);
    return pk;
  } else {
    return address;
  }
}

/**
 * An address to identify an account on chain. Can be a [[PublicKey]],
 * or Base 58 encoded string.
 */
export type Address = PublicKey | string;
