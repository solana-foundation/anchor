import { Layout } from "buffer-layout";
import { Idl } from "../idl";
import { IdlCoder } from "./idl";

/**
 * Encodes and decodes user defined types.
 */
export class TypesCoder {
  /**
   * Maps account type identifier to a layout.
   */
  private layouts: Map<string, Layout>;

  public constructor(idl: Idl) {
    if (idl.types === undefined) {
      this.layouts = new Map();
      return;
    }
    const layouts = idl.types.map((acc) => {
      return [acc.name, IdlCoder.typeDefLayout(acc, idl.types)];
    });

    // @ts-ignore
    this.layouts = new Map(layouts);
  }

  public encode<T = any>(accountName: string, account: T): Buffer {
    const buffer = Buffer.alloc(1000); // TODO: use a tighter buffer.
    const layout = this.layouts.get(accountName);
    const len = layout.encode(account, buffer);
    return buffer.slice(0, len);
  }

  public decode<T = any>(accountName: string, ix: Buffer): T {
    const layout = this.layouts.get(accountName);
    return layout.decode(ix);
  }
}
