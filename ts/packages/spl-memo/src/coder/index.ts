import { Idl, Coder } from "@coral-xyz/anchor";

import { SplMemoAccountsCoder } from "./accounts";
import { SplMemoEventsCoder } from "./events";
import { SplMemoInstructionCoder } from "./instructions";
import { SplMemoStateCoder } from "./state";
import { SplMemoTypesCoder } from "./types";

/**
 * Coder for SplMemo
 */
export class SplMemoCoder implements Coder {
  readonly accounts: SplMemoAccountsCoder;
  readonly events: SplMemoEventsCoder;
  readonly instruction: SplMemoInstructionCoder;
  readonly state: SplMemoStateCoder;
  readonly types: SplMemoTypesCoder;

  constructor(idl: Idl) {
    this.accounts = new SplMemoAccountsCoder(idl);
    this.events = new SplMemoEventsCoder(idl);
    this.instruction = new SplMemoInstructionCoder(idl);
    this.state = new SplMemoStateCoder(idl);
    this.types = new SplMemoTypesCoder(idl);
  }
}
