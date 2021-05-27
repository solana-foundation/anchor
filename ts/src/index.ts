import BN from "bn.js";
import * as web3 from "@solana/web3.js";
import Provider, { NodeWallet as Wallet } from "./provider";
import Coder, {
  InstructionCoder,
  EventCoder,
  StateCoder,
  TypesCoder,
} from "./coder";
import { Idl } from "./idl";
import workspace from "./workspace";
import * as utils from "./utils";
import { Program } from "./program";
import { Address } from "./program/common";
import { Event } from "./program/event";
import {
  ProgramAccount,
  AccountNamespace,
  AccountClient,
  StateClient,
  RpcNamespace,
  RpcFn,
  SimulateNamespace,
  SimulateFn,
  TransactionNamespace,
  TransactionFn,
  InstructionNamespace,
  InstructionFn,
} from "./program/namespace";
import { Context, Accounts } from "./program/context";

let _provider: Provider | null = null;

/**
 * Sets the default provider on the client.
 */
function setProvider(provider: Provider) {
  _provider = provider;
}

/**
 * Returns the default provider being used by the client.
 */
function getProvider(): Provider {
  if (_provider === null) {
    return Provider.local();
  }
  return _provider;
}

export {
  workspace,
  Program,
  AccountNamespace,
  AccountClient,
  StateClient,
  RpcNamespace,
  RpcFn,
  SimulateNamespace,
  SimulateFn,
  TransactionNamespace,
  TransactionFn,
  InstructionNamespace,
  InstructionFn,
  ProgramAccount,
  Context,
  Accounts,
  Coder,
  InstructionCoder,
  EventCoder,
  StateCoder,
  TypesCoder,
  Event,
  setProvider,
  getProvider,
  Provider,
  BN,
  web3,
  Idl,
  utils,
  Wallet,
  Address,
};
