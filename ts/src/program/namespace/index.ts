import camelCase from "camelcase";
import { PublicKey } from "@solana/web3.js";
import Coder from "../../coder";
import Provider from "../../provider";
import { Idl } from "../../idl";
import { parseIdlErrors } from "../common";
import StateNamespace, { State } from "./state";
import InstructionNamespace, { Ixs } from "./instruction";
import TransactionNamespace, { Txs } from "./transaction";
import RpcNamespace, { Rpcs } from "./rpc";
import AccountNamespace, { Accounts } from "./account";

// Re-exports.
export { State } from "./state";
export { Ixs } from "./instruction";
export { Txs, TxFn } from "./transaction";
export { Rpcs, RpcFn } from "./rpc";
export { Accounts, AccountFn, ProgramAccount } from "./account";

export default class NamespaceFactory {
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
    const state = StateNamespace.build(
      idl,
      coder,
      programId,
      idlErrors,
      provider
    );

    idl.instructions.forEach((idlIx) => {
      const ix = InstructionNamespace.build(idlIx, coder, programId);
      const tx = TransactionNamespace.build(idlIx, ix);
      const rpc = RpcNamespace.build(idlIx, tx, idlErrors, provider);

      const name = camelCase(idlIx.name);

      ixFns[name] = ix;
      txFns[name] = tx;
      rpcs[name] = rpc;
    });

    const accountFns = idl.accounts
      ? AccountNamespace.build(idl, coder, programId, provider)
      : {};

    return [rpcs, ixFns, txFns, accountFns, state];
  }
}
