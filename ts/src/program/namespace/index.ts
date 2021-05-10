import camelCase from "camelcase";
import { PublicKey } from "@solana/web3.js";
import Coder from "../../coder";
import Provider from "../../provider";
import { Idl } from "../../idl";
import { parseIdlErrors } from "../common";
import StateFactory, { StateNamespace } from "./state";
import InstructionFactory, { InstructionNamespace } from "./instruction";
import TransactionFactory, { TransactionNamespace } from "./transaction";
import RpcFactory, { RpcNamespace } from "./rpc";
import AccountFactory, { AccountNamespace } from "./account";
import SimulateFactory, { SimulateNamespace } from "./simulate";

// Re-exports.
export { StateNamespace } from "./state";
export { InstructionNamespace } from "./instruction";
export { TransactionNamespace, TxFn } from "./transaction";
export { RpcNamespace, RpcFn } from "./rpc";
export { AccountNamespace, AccountFn, ProgramAccount } from "./account";
export { SimulateNamespace } from "./simulate";

export default class NamespaceFactory {
  /**
   * Generates all namespaces for a given program.
   */
  public static build(
    idl: Idl,
    coder: Coder,
    programId: PublicKey,
    provider: Provider
  ): [
    RpcNamespace,
    InstructionNamespace,
    TransactionNamespace,
    AccountNamespace,
    StateNamespace,
    SimulateNamespace
  ] {
    const idlErrors = parseIdlErrors(idl);

    const rpc: RpcNamespace = {};
    const instruction: InstructionNamespace = {};
    const transaction: TransactionNamespace = {};
    const simulate: SimulateNamespace = {};

    const state = StateFactory.build(
      idl,
      coder,
      programId,
      idlErrors,
      provider
    );

    idl.instructions.forEach((idlIx) => {
      const ixItem = InstructionFactory.build(idlIx, coder, programId);
      const txItem = TransactionFactory.build(idlIx, ixItem);
      const rpcItem = RpcFactory.build(idlIx, txItem, idlErrors, provider);
      const simulateItem = SimulateFactory.build(
        idlIx,
        txItem,
        idlErrors,
        provider,
        coder,
        programId,
        idl
      );

      const name = camelCase(idlIx.name);

      instruction[name] = ixItem;
      transaction[name] = txItem;
      rpc[name] = rpcItem;
      simulate[name] = simulateItem;
    });

    const account = idl.accounts
      ? AccountFactory.build(idl, coder, programId, provider)
      : {};

    return [rpc, instruction, transaction, account, state, simulate];
  }
}
