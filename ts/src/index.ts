import { isBrowser } from "./utils/common.js";

export { default as BN } from "bn.js";
export * as web3 from "@solana/web3.js";
export { default as Provider, getProvider, setProvider } from "./provider.js";
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

export * as utils from "./utils/index.js";
export * from "./program/index.js";

if (!isBrowser) {
  exports.workspace = require("./workspace.js").default;
  exports.Wallet = require("./nodewallet.js").default;
}
