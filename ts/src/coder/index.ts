import { Idl } from "../idl.js";
import { InstructionCoder } from "./instruction.js";
import { AccountsCoder } from "./accounts.js";
import { EventCoder } from "./event.js";
import { StateCoder } from "./state.js";
import { sighash } from "./common.js";

export { accountSize } from "./common.js";
export { InstructionCoder } from "./instruction.js";
export { AccountsCoder, ACCOUNT_DISCRIMINATOR_SIZE } from "./accounts.js";
export { EventCoder, eventDiscriminator } from "./event.js";
export { StateCoder, stateDiscriminator } from "./state.js";

/**
 * Coder provides a facade for encoding and decoding all IDL related objects.
 */
export default class Coder<A extends string = string> {
  /**
   * Instruction coder.
   */
  readonly instruction: InstructionCoder;

  /**
   * Account coder.
   */
  readonly accounts: AccountsCoder<A>;

  /**
   * Coder for state structs.
   */
  readonly state: StateCoder;

  /**
   * Coder for events.
   */
  readonly events: EventCoder;

  constructor(idl: Idl) {
    this.instruction = new InstructionCoder(idl);
    this.accounts = new AccountsCoder(idl);
    this.events = new EventCoder(idl);
    if (idl.state) {
      this.state = new StateCoder(idl);
    }
  }

  public sighash(nameSpace: string, ixName: string): Buffer {
    return sighash(nameSpace, ixName);
  }
}
