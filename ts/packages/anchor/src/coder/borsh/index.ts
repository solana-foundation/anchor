import { Idl } from "../../idl.js";
import { BorshInstructionCoder } from "./instruction.js";
import { BorshAccountsCoder } from "./accounts.js";
import { BorshEventCoder } from "./event.js";
import { BorshTypesCoder } from "./types.js";
import { WincodeInstructionCoder } from "../wincode/instruction.js";
import { Coder, InstructionCoder } from "../index.js";

export { BorshInstructionCoder } from "./instruction.js";
export { BorshAccountsCoder } from "./accounts.js";
export { BorshEventCoder } from "./event.js";

/**
 * BorshCoder is the default Coder for Anchor v2 programs.
 *
 * Account + event + user-type decoding go through borsh layouts — that
 * matches the on-disk format for v2 accounts and borsh-mode events.
 * Instruction args are wincode-encoded on v2 (the derive emits
 * `wincode::deserialize` in every handler), so the instruction coder
 * defaults to {@link WincodeInstructionCoder}. Callers wanting v1
 * compatibility can pass an explicit {@link BorshInstructionCoder}.
 */
export class BorshCoder<A extends string = string, T extends string = string>
  implements Coder
{
  /** Instruction coder. */
  readonly instruction: InstructionCoder;

  /** Account coder. */
  readonly accounts: BorshAccountsCoder<A>;

  /** Event coder. */
  readonly events: BorshEventCoder;

  /** User-defined types coder. */
  readonly types: BorshTypesCoder<T>;

  constructor(idl: Idl, instruction?: InstructionCoder) {
    this.instruction = instruction ?? new WincodeInstructionCoder(idl);
    this.accounts = new BorshAccountsCoder(idl);
    this.events = new BorshEventCoder(idl);
    this.types = new BorshTypesCoder(idl);
  }
}
