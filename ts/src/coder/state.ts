import { Buffer } from "buffer";
import { Layout } from "buffer-layout";
import { sha256 } from "js-sha256";
import { Idl } from "../idl.js";
import { IdlCoder } from "./idl.js";
import * as features from "../utils/features.js";

export class StateCoder {
  private layout: Layout;

  public constructor(idl: Idl) {
    if (idl.state === undefined) {
      throw new Error("Idl state not defined.");
    }
    this.layout = IdlCoder.typeDefLayout(idl.state.struct, idl.types);
  }

  public async encode<T = any>(name: string, account: T): Promise<Buffer> {
    const buffer = Buffer.alloc(1000); // TODO: use a tighter buffer.
    const len = this.layout.encode(account, buffer);

    const disc = await stateDiscriminator(name);
    const accData = buffer.slice(0, len);

    return Buffer.concat([disc, accData]);
  }

  public decode<T = any>(ix: Buffer): T {
    // Chop off discriminator.
    const data = ix.slice(8);
    return this.layout.decode(data);
  }
}

// Calculates unique 8 byte discriminator prepended to all anchor state accounts.
export async function stateDiscriminator(name: string): Promise<Buffer> {
  let ns = features.isSet("anchor-deprecated-state") ? "account" : "state";
  return Buffer.from(sha256.digest(`${ns}:${name}`)).slice(0, 8);
}
