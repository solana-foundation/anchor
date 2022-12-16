import { Idl, Coder } from "@coral-xyz/anchor";

import { SplNameServiceAccountsCoder } from "./accounts";
import { SplNameServiceEventsCoder } from "./events";
import { SplNameServiceInstructionCoder } from "./instructions";
import { SplNameServiceStateCoder } from "./state";
import { SplNameServiceTypesCoder } from "./types";

/**
 * Coder for SplNameService
 */
export class SplNameServiceCoder implements Coder {
  readonly accounts: SplNameServiceAccountsCoder;
  readonly events: SplNameServiceEventsCoder;
  readonly instruction: SplNameServiceInstructionCoder;
  readonly state: SplNameServiceStateCoder;
  readonly types: SplNameServiceTypesCoder;

  constructor(idl: Idl) {
    this.accounts = new SplNameServiceAccountsCoder(idl);
    this.events = new SplNameServiceEventsCoder(idl);
    this.instruction = new SplNameServiceInstructionCoder(idl);
    this.state = new SplNameServiceStateCoder(idl);
    this.types = new SplNameServiceTypesCoder(idl);
  }
}
