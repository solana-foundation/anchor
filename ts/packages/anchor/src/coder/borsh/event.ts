import { Buffer } from "buffer";
import { Layout } from "buffer-layout";
import * as base64 from "../../utils/bytes/base64.js";
import {
  Idl,
  IdlDiscriminator,
  IdlField,
  IdlType,
  IdlTypeDef,
} from "../../idl.js";
import { IdlCoder } from "./idl.js";
import { EventCoder } from "../index.js";

// Named fields come through as `IdlField` (object with `.type`), tuple
// fields come through directly as `IdlType` (which may be a bare string like
// `"u8"`). Normalize to the inner type.
function fieldInnerType(f: IdlField | IdlType): IdlType {
  if (typeof f === "object" && !Array.isArray(f) && "type" in f) {
    return (f as IdlField).type;
  }
  return f as IdlType;
}

/**
 * Walk an IDL type looking for fields whose wincode and borsh wire encodings
 * diverge. The only divergence for primitive/compound types is the length
 * prefix on `Vec` / `String` / `bytes`: wincode writes it as u64 LE, borsh as
 * u32 LE. Everything else (fixed primitives, fixed arrays, `Option`'s tag
 * byte + payload, enum tag byte + payload) matches bit-for-bit. So a wincode
 * event with only fixed-width fields can be safely decoded by the borsh
 * layout, but one containing a `Vec` or `String` (directly or transitively
 * through a defined struct) cannot.
 */
function wincodeDivergesFromBorsh(
  ty: IdlType,
  types: IdlTypeDef[],
  seen: Set<string>,
): boolean {
  if (typeof ty === "string") {
    return ty === "string" || ty === "bytes";
  }
  if ("vec" in ty) return true;
  if ("option" in ty) return wincodeDivergesFromBorsh(ty.option, types, seen);
  if ("coption" in ty)
    return wincodeDivergesFromBorsh(ty.coption, types, seen);
  if ("array" in ty) return wincodeDivergesFromBorsh(ty.array[0], types, seen);
  if ("defined" in ty) {
    const name = ty.defined.name;
    if (seen.has(name)) return false;
    seen.add(name);
    const td = types.find((t) => t.name === name);
    if (!td) return false;
    if (td.type.kind === "struct") {
      const fields = td.type.fields ?? [];
      for (const f of fields) {
        const inner = fieldInnerType(f);
        if (wincodeDivergesFromBorsh(inner, types, seen)) return true;
      }
      return false;
    }
    if (td.type.kind === "enum") {
      for (const v of td.type.variants) {
        const fields = v.fields ?? [];
        for (const f of fields) {
          const inner: IdlType = fieldInnerType(f);
          if (wincodeDivergesFromBorsh(inner, types, seen)) return true;
        }
      }
      return false;
    }
    if (td.type.kind === "type") {
      return wincodeDivergesFromBorsh(td.type.alias, types, seen);
    }
  }
  return false;
}

export class BorshEventCoder implements EventCoder {
  /**
   * Maps event name to its layout plus a guard explaining why it can't be
   * decoded (if any). We still build the layout for wincode events with
   * fixed fields — wire-compatible with borsh — so the common case works
   * transparently; only wincode events carrying `Vec`/`String` get a
   * deferred error.
   */
  private layouts: Map<
    string,
    {
      discriminator: IdlDiscriminator;
      layout: Layout;
      unsupported?: string;
    }
  >;

  public constructor(idl: Idl) {
    if (!idl.events) {
      this.layouts = new Map();
      return;
    }

    const types = idl.types;
    if (!types) {
      throw new Error("Events require `idl.types`");
    }

    const layouts = idl.events.map((ev) => {
      const typeDef = types.find((ty) => ty.name === ev.name);
      if (!typeDef) {
        throw new Error(`Event not found: ${ev.name}`);
      }
      let unsupported: string | undefined;
      // Accept both the first-class `"wincode"` string and the forward-compat
      // `{custom: "wincode"}` escape-hatch shape. The derive emits the latter
      // today for surfpool compat; this reader handles either.
      const ser = typeDef.serialization;
      const isWincode =
        ser === "wincode" ||
        (typeof ser === "object" &&
          ser !== null &&
          "custom" in ser &&
          ser.custom === "wincode");
      if (isWincode && typeDef.type.kind === "struct") {
        const fields = typeDef.type.fields ?? [];
        const seen = new Set<string>();
        for (const f of fields) {
          const inner: IdlType = fieldInnerType(f);
          if (wincodeDivergesFromBorsh(inner, types, seen)) {
            unsupported =
              `event \`${ev.name}\` is wincode-serialized and contains ` +
              `Vec/String/bytes fields whose wire format diverges from ` +
              `borsh (u64 vs u32 length prefix). A wincode JS decoder is ` +
              `not yet implemented — please decode manually or tag the ` +
              `event \`#[event(borsh)]\` for off-chain compatibility.`;
            break;
          }
        }
      }
      return [
        ev.name,
        {
          discriminator: ev.discriminator,
          layout: IdlCoder.typeDefLayout({ typeDef, types }),
          unsupported,
        },
      ] as const;
    });
    this.layouts = new Map(layouts);
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

    for (const [name, layout] of this.layouts) {
      const givenDisc = logArr.subarray(0, layout.discriminator.length);
      const matches = givenDisc.equals(Buffer.from(layout.discriminator));
      if (matches) {
        if (layout.unsupported) {
          throw new Error(layout.unsupported);
        }
        return {
          name,
          data: layout.layout.decode(logArr.subarray(givenDisc.length)),
        };
      }
    }

    return null;
  }
}
