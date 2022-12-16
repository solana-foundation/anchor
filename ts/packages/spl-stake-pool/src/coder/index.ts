import { Idl, Coder } from "@coral-xyz/anchor";

import { SplStakePoolAccountsCoder } from "./accounts";
import { SplStakePoolEventsCoder } from "./events";
import { SplStakePoolInstructionCoder } from "./instructions";
import { SplStakePoolStateCoder } from "./state";
import { SplStakePoolTypesCoder } from "./types";

/**
 * Coder for SplStakePool
 */
export class SplStakePoolCoder implements Coder {
  readonly accounts: SplStakePoolAccountsCoder;
  readonly events: SplStakePoolEventsCoder;
  readonly instruction: SplStakePoolInstructionCoder;
  readonly state: SplStakePoolStateCoder;
  readonly types: SplStakePoolTypesCoder;

  constructor(idl: Idl) {
    this.accounts = new SplStakePoolAccountsCoder(idl);
    this.events = new SplStakePoolEventsCoder(idl);
    this.instruction = new SplStakePoolInstructionCoder(idl);
    this.state = new SplStakePoolStateCoder(idl);
    this.types = new SplStakePoolTypesCoder(idl);
  }
}
