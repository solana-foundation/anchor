import { Buffer } from "buffer";
import { Layout } from "buffer-layout";
import * as base64 from "../../utils/bytes/base64.js";
import { Idl } from "../../idl.js";
import { IdlCoder } from "./idl.js";
import { EventCoder } from "../index.js";

export class BorshEventCoder implements EventCoder {
  /**
   * Maps account type identifier to a layout.
   */
  private layouts: Map<string, Layout>;

  /**
   * Maps base64 encoded event discriminator to event name.
   */
  private discriminators: Map<string, string>;

  public constructor(idl: Idl) {
    if (!idl.events) {
      this.layouts = new Map();
      return;
    }

    const types = idl.types;
    if (!types) {
      throw new Error("Events require `idl.types`");
    }

    const layouts: [string, Layout<any>][] = idl.events.map((ev) => {
      const typeDef = types.find((ty) => ty.name === ev.name);
      if (!typeDef) {
        throw new Error(`Event not found: ${ev.name}`);
      }
      return [ev.name, IdlCoder.typeDefLayout({ typeDef, types })];
    });
    this.layouts = new Map(layouts);

    this.discriminators = new Map<string, string>(
      (idl.events ?? []).map((ev) => [
        base64.encode(Buffer.from(ev.discriminator)),
        ev.name,
      ])
    );
  }

  public decode(log: string): {
    name: string;
    data: any;
  } | null {
    let logArr: Buffer;
    // This will throw if log length is not a multiple of 4.
    try {
      logArr = base64.decode(log);
    } catch (e) {
      return null;
    }
    const disc = base64.encode(logArr.slice(0, 8));

    // Only deserialize if the discriminator implies a proper event.
    const eventName = this.discriminators.get(disc);
    if (!eventName) {
      return null;
    }

    const layout = this.layouts.get(eventName);
    if (!layout) {
      throw new Error(`Unknown event: ${eventName}`);
    }
    const data = layout.decode(logArr.slice(8));
    return { data, name: eventName };
  }
}
