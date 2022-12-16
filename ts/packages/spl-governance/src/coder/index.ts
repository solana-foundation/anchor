import { Idl, Coder } from "@coral-xyz/anchor";

import { SplGovernanceAccountsCoder } from "./accounts";
import { SplGovernanceEventsCoder } from "./events";
import { SplGovernanceInstructionCoder } from "./instructions";
import { SplGovernanceStateCoder } from "./state";
import { SplGovernanceTypesCoder } from "./types";

/**
 * Coder for SplGovernance
 */
export class SplGovernanceCoder implements Coder {
  readonly accounts: SplGovernanceAccountsCoder;
  readonly events: SplGovernanceEventsCoder;
  readonly instruction: SplGovernanceInstructionCoder;
  readonly state: SplGovernanceStateCoder;
  readonly types: SplGovernanceTypesCoder;

  constructor(idl: Idl) {
    this.accounts = new SplGovernanceAccountsCoder(idl);
    this.events = new SplGovernanceEventsCoder(idl);
    this.instruction = new SplGovernanceInstructionCoder(idl);
    this.state = new SplGovernanceStateCoder(idl);
    this.types = new SplGovernanceTypesCoder(idl);
  }
}
