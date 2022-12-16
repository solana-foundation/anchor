import { Idl, Coder } from "@coral-xyz/anchor";

import { SplTokenSwapAccountsCoder } from "./accounts";
import { SplTokenSwapEventsCoder } from "./events";
import { SplTokenSwapInstructionCoder } from "./instructions";
import { SplTokenSwapStateCoder } from "./state";
import { SplTokenSwapTypesCoder } from "./types";

/**
 * Coder for SplTokenSwap
 */
export class SplTokenSwapCoder implements Coder {
  readonly accounts: SplTokenSwapAccountsCoder;
  readonly events: SplTokenSwapEventsCoder;
  readonly instruction: SplTokenSwapInstructionCoder;
  readonly state: SplTokenSwapStateCoder;
  readonly types: SplTokenSwapTypesCoder;

  constructor(idl: Idl) {
    this.accounts = new SplTokenSwapAccountsCoder(idl);
    this.events = new SplTokenSwapEventsCoder(idl);
    this.instruction = new SplTokenSwapInstructionCoder(idl);
    this.state = new SplTokenSwapStateCoder(idl);
    this.types = new SplTokenSwapTypesCoder(idl);
  }
}
