import { Idl } from "../../idl.js";
import { Coder } from "../index.js";
import { SystemInstructionCoder } from "./instruction.js";
import { SystemStateCoder } from "./state.js";
import { SystemAccountsCoder } from "./accounts.js";
import { SystemEventsCoder } from "./events.js";

/**
 * Coder for the System program.
 */
export class SystemCoder implements Coder {
  readonly instruction: SystemInstructionCoder;
  readonly accounts: SystemAccountsCoder;
  readonly state: SystemStateCoder;
  readonly events: SystemEventsCoder;

  constructor(idl: Idl) {
    this.instruction = new SystemInstructionCoder(idl);
    this.accounts = new SystemAccountsCoder(idl);
    this.events = new SystemEventsCoder(idl);
    this.state = new SystemStateCoder(idl);
  }
}
