import camelCase from "camelcase";
import { Idl } from "../../idl.js";
import { InstructionCoder } from "../index.js";

export class SplAssociatedTokenInstructionCoder implements InstructionCoder {
  constructor(_: Idl) {}

  encode(ixName: string, _: any): Buffer {
    switch (camelCase(ixName)) {
      case "create": {
        return Buffer.alloc(0);
      }
      default: {
        throw new Error(`Invalid instruction: ${ixName}`);
      }
    }
  }

  encodeState(_ixName: string, _ix: any): Buffer {
    throw new Error("SPL associated token does not have state");
  }
}
