export { default as BN } from "bn.js";
export * as web3 from "@solana/web3.js";
export {
  default as Provider,
  getProvider,
  setProvider,
  NodeWallet as Wallet,
} from "./provider.js";
export {
  default as Coder,
  InstructionCoder,
  EventCoder,
  StateCoder,
  AccountsCoder,
} from "./coder/index.js";

export * from "./error.js";
export { Instruction } from "./coder/instruction.js";
export { Idl } from "./idl.js";
export { default as workspace } from "./workspace.js";
export * as utils from "./utils/index.js";
export * from "./program/index.js";
