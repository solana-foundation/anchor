import camelCase from "camelcase";
import { PublicKey } from "@solana/web3.js";
import Coder from "../../coder";
import Provider from "../../provider";
import { Idl } from "../../idl";
import StateFactory, { StateClient } from "./state";
import InstructionFactory, { InstructionNamespace } from "./instruction";
import TransactionFactory, { TransactionNamespace } from "./transaction";
import RpcFactory, { RpcNamespace } from "./rpc";
import AccountFactory, { AccountNamespace } from "./account";
import SimulateFactory, { SimulateNamespace } from "./simulate";
import { parseIdlErrors } from "../common";

// Re-exports.
export { StateClient } from "./state";
export { InstructionNamespace, InstructionFn } from "./instruction";
export { TransactionNamespace, TransactionFn } from "./transaction";
export { RpcNamespace, RpcFn } from "./rpc";
export { AccountNamespace, AccountClient, ProgramAccount } from "./account";
export { SimulateNamespace, SimulateFn } from "./simulate";

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
    SimulateNamespace,
    StateClient | undefined
  ] {
    const rpc: RpcNamespace = {};
    const instruction: InstructionNamespace = {};
    const transaction: TransactionNamespace = {};
    const simulate: SimulateNamespace = {};

    const idlErrors = parseIdlErrors(idl);

    const state = StateFactory.build(idl, coder, programId, provider);

    idl.instructions.forEach((idlIx) => {
      const ixItem = InstructionFactory.build(
        idlIx,
        (ixName: string, ix: any) => coder.instruction.encode(ixName, ix),
        programId
      );
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

    return [rpc, instruction, transaction, account, simulate, state];
  }
}
