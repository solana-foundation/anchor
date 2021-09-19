export { default as BN } from "bn.js";
export * as web3 from "@solana/web3.js";
export {
  default as Provider,
  getProvider,
  setProvider,
  NodeWallet as Wallet,
} from "./provider";
export {
  default as Coder,
  InstructionCoder,
  EventCoder,
  StateCoder,
  TypesCoder,
  AccountsCoder,
} from "./coder";

export * from "./error";
export { Instruction } from "./coder/instruction";
export { Idl } from "./idl";
export { default as workspace } from "./workspace";
export * as utils from "./utils";
export * from "./program";
