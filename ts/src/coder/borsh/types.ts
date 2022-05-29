import { Buffer } from "buffer";
import { Layout } from "buffer-layout";
import { Idl } from "../../idl.js";
import { IdlCoder } from "./idl.js";
import { TypesCoder } from "../index.js";

/**
 * Encodes and decodes user-defined types.
 */
export class BorshTypesCoder<N extends string = string> implements TypesCoder {
  /**
   * Maps type name to a layout.
   */
  private typeLayouts: Map<N, Layout>;

  /**
   * IDL whose types will be coded.
   */
  private idl: Idl;

  public constructor(idl: Idl) {
    if (idl.types === undefined) {
      this.typeLayouts = new Map();
      return;
    }
    const layouts: [N, Layout][] = idl.types.map((acc) => {
      return [acc.name as N, IdlCoder.typeDefLayout(acc, idl.types)];
    });

    this.typeLayouts = new Map(layouts);
    this.idl = idl;
  }

  public encode<T = any>(typeName: N, type: T): Buffer {
    const buffer = Buffer.alloc(1000); // TODO: use a tighter buffer.
    const layout = this.typeLayouts.get(typeName);
    if (!layout) {
      throw new Error(`Unknown type: ${typeName}`);
    }
    const len = layout.encode(type, buffer);

    return buffer.slice(0, len);
  }

  public decode<T = any>(typeName: N, typeData: Buffer): T {
    const layout = this.typeLayouts.get(typeName);
    if (!layout) {
      throw new Error(`Unknown type: ${typeName}`);
    }
    return layout.decode(typeData);
  }
}
