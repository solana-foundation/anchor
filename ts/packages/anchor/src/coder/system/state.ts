import { StateCoder } from "../index.js";
import { Idl } from "../../idl";

export class SystemStateCoder implements StateCoder {
  // eslint-disable-next-line @typescript-eslint/no-empty-function
  constructor(_idl: Idl) {}

  encode<T = any>(_name: string, _account: T): Promise<Buffer> {
    throw new Error("System does not have state");
  }
  decode<T = any>(_ix: Buffer): T {
    throw new Error("System does not have state");
  }
}
