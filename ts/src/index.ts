import BN from "bn.js";
import * as web3 from "@solana/web3.js";
import Provider, {
  getProvider,
  setProvider,
  NodeWallet as Wallet,
} from "./provider";
import Coder, {
  InstructionCoder,
  EventCoder,
  StateCoder,
  TypesCoder,
  AccountsCoder,
} from "./coder";
import { Instruction } from "./coder/instruction";
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
import { EventParser } from "./program/event";

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
  AccountsCoder,
  Event,
  Instruction,
  setProvider,
  getProvider,
  Provider,
  BN,
  web3,
  Idl,
  utils,
  Wallet,
  Address,
  EventParser,
};
