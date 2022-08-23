import camelCase from "camelcase";
import { PublicKey } from "@solana/web3.js";
import { Coder } from "../../coder/index.js";
import Provider from "../../provider.js";
import { Idl } from "../../idl.js";
import StateFactory, { StateClient } from "./state.js";
import InstructionFactory, { InstructionNamespace } from "./instruction.js";
import TransactionFactory, { TransactionNamespace } from "./transaction.js";
import RpcFactory, { RpcNamespace } from "./rpc.js";
import AccountFactory, { AccountNamespace } from "./account.js";
import SimulateFactory, { SimulateNamespace } from "./simulate.js";
import { parseIdlErrors } from "../common.js";
import { MethodsBuilderFactory, MethodsNamespace } from "./methods";
import ViewFactory, { ViewNamespace } from "./views";

// Re-exports.
export { StateClient } from "./state.js";
export { InstructionNamespace, InstructionFn } from "./instruction.js";
export { TransactionNamespace, TransactionFn } from "./transaction.js";
export { RpcNamespace, RpcFn } from "./rpc.js";
export { AccountNamespace, AccountClient, ProgramAccount } from "./account.js";
export { SimulateNamespace, SimulateFn } from "./simulate.js";
export { IdlAccounts, IdlTypes } from "./types.js";
export { MethodsBuilderFactory, MethodsNamespace } from "./methods";
export { ViewNamespace, ViewFn } from "./views";

export default class NamespaceFactory {
  /**
   * Generates all namespaces for a given program.
   */
  public static build<IDL extends Idl>(
    idl: IDL,
    coder: Coder,
    programId: PublicKey,
    provider: Provider
  ): [
    RpcNamespace<IDL>,
    InstructionNamespace<IDL>,
    TransactionNamespace<IDL>,
    AccountNamespace<IDL>,
    SimulateNamespace<IDL>,
    MethodsNamespace<IDL>,
    StateClient<IDL> | undefined,
    ViewNamespace<IDL> | undefined
  ] {
    const rpc: RpcNamespace = {};
    const instruction: InstructionNamespace = {};
    const transaction: TransactionNamespace = {};
    const simulate: SimulateNamespace = {};
    const methods: MethodsNamespace = {};
    const view: ViewNamespace = {};

    const idlErrors = parseIdlErrors(idl);

    const account: AccountNamespace<IDL> = idl.accounts
      ? AccountFactory.build(idl, coder, programId, provider)
      : ({} as AccountNamespace<IDL>);

    const state = StateFactory.build(idl, coder, programId, provider);

    idl.instructions.forEach((idlIx) => {
      const ixItem = InstructionFactory.build<IDL, typeof idlIx>(
        idlIx,
        (ixName, ix) => coder.instruction.encode(ixName, ix),
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
      const viewItem = ViewFactory.build(programId, idlIx, simulateItem, idl);
      const methodItem = MethodsBuilderFactory.build<IDL, typeof idlIx>(
        provider,
        programId,
        idlIx,
        ixItem,
        txItem,
        rpcItem,
        simulateItem,
        viewItem,
        account
      );
      const name = camelCase(idlIx.name);

      instruction[name] = ixItem;
      transaction[name] = txItem;
      rpc[name] = rpcItem;
      simulate[name] = simulateItem;
      methods[name] = methodItem;
      if (viewItem) {
        view[name] = viewItem;
      }
    });

    return [
      rpc as RpcNamespace<IDL>,
      instruction as InstructionNamespace<IDL>,
      transaction as TransactionNamespace<IDL>,
      account,
      simulate as SimulateNamespace<IDL>,
      methods as MethodsNamespace<IDL>,
      state,
      view as ViewNamespace<IDL>,
    ];
  }
}
