import { Idl } from "../idl";
import { InstructionCoder } from "./instruction";
import { AccountsCoder } from "./accounts";
import { TypesCoder } from "./types";
import { EventCoder } from "./event";
import { StateCoder } from "./state";
import { sighash } from "./common";

export { accountSize } from "./common";
export { TypesCoder } from "./types";
export { InstructionCoder } from "./instruction";
export { AccountsCoder, ACCOUNT_DISCRIMINATOR_SIZE } from "./accounts";
export { EventCoder, eventDiscriminator } from "./event";
export { StateCoder, stateDiscriminator } from "./state";

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
   * Types coder.
   */
  readonly types: TypesCoder;

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
    this.types = new TypesCoder(idl);
    this.events = new EventCoder(idl);
    if (idl.state) {
      this.state = new StateCoder(idl);
    }
  }

  public sighash(nameSpace: string, ixName: string): Buffer {
    return sighash(nameSpace, ixName);
  }
}
