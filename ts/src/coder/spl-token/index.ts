import { Idl } from "../../idl.js";
import { Coder } from "../index.js";
import { SplTokenInstructionCoder } from "./instruction.js";
import { SplTokenStateCoder } from "./state.js";
import { SplTokenAccountsCoder } from "./accounts.js";
import { SplTokenEventsCoder } from "./events.js";

/**
 * Coder for the SPL token program.
 */
export class SplTokenCoder implements Coder {
  readonly instruction: SplTokenInstructionCoder;
  readonly accounts: SplTokenAccountsCoder;
  readonly state: SplTokenStateCoder;
  readonly events: SplTokenEventsCoder;

  constructor(idl: Idl) {
    this.instruction = new SplTokenInstructionCoder(idl);
    this.accounts = new SplTokenAccountsCoder(idl);
    this.events = new SplTokenEventsCoder(idl);
    this.state = new SplTokenStateCoder(idl);
  }
}
