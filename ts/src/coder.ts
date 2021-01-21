import camelCase from "camelcase";
import { Layout } from "buffer-layout";
import * as borsh from "@project-serum/borsh";
import { Idl, IdlField, IdlTypeDef, IdlEnumVariant, IdlType } from "./idl";
import { IdlError } from "./error";

/**
 * Coder provides a facade for encoding and decoding all IDL related objects.
 */
export default class Coder {
  /**
   * Instruction coder.
   */
  readonly instruction: InstructionCoder;

  /**
   * Account coder.
   */
  readonly accounts: AccountsCoder;

  /**
   * Types coder.
   */
  readonly types: TypesCoder;

  constructor(idl: Idl) {
    this.instruction = new InstructionCoder(idl);
    this.accounts = new AccountsCoder(idl);
    this.types = new TypesCoder(idl);
  }
}

/**
 * Encodes and decodes program instructions.
 */
class InstructionCoder<T = any> {
  /**
   * Instruction enum layout.
   */
  private ixLayout: Layout;

  public constructor(idl: Idl) {
    this.ixLayout = InstructionCoder.parseIxLayout(idl);
  }

  public encode(ix: T): Buffer {
    const buffer = Buffer.alloc(1000); // TODO: use a tighter buffer.
    const len = this.ixLayout.encode(ix, buffer);
    return buffer.slice(0, len);
  }

  public decode(ix: Buffer): T {
    return this.ixLayout.decode(ix);
  }

  private static parseIxLayout(idl: Idl): Layout {
    let ixLayouts = idl.instructions.map((ix) => {
      let fieldLayouts = ix.args.map((arg: IdlField) =>
        IdlCoder.fieldLayout(arg, idl.types)
      );
      const name = camelCase(ix.name);
      return borsh.struct(fieldLayouts, name);
    });
    return borsh.rustEnum(ixLayouts);
  }
}

/**
 * Encodes and decodes account objects.
 */
class AccountsCoder {
  /**
   * Maps account type identifier to a layout.
   */
  private accountLayouts: Map<string, Layout>;

  public constructor(idl: Idl) {
    if (idl.accounts === undefined) {
      this.accountLayouts = new Map();
      return;
    }
    const layouts = idl.accounts.map((acc) => {
      return [acc.name, IdlCoder.typeDefLayout(acc, idl.types)];
    });

    // @ts-ignore
    this.accountLayouts = new Map(layouts);
  }

  public encode<T = any>(accountName: string, account: T): Buffer {
    const buffer = Buffer.alloc(1000); // TODO: use a tighter buffer.
    const layout = this.accountLayouts.get(accountName);
    const len = layout.encode(account, buffer);
    return buffer.slice(0, len);
  }

  public decode<T = any>(accountName: string, ix: Buffer): T {
    const layout = this.accountLayouts.get(accountName);
    return layout.decode(ix);
  }
}

/**
 * Encodes and decodes user defined types.
 */
class TypesCoder {
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

class IdlCoder {
  public static fieldLayout(field: IdlField, types?: IdlTypeDef[]): Layout {
    const fieldName =
      field.name !== undefined ? camelCase(field.name) : undefined;
    switch (field.type) {
      case "bool": {
        return borsh.bool(fieldName);
      }
      case "u8": {
        return borsh.u8(fieldName);
      }
      case "u32": {
        return borsh.u32(fieldName);
      }
      case "u64": {
        return borsh.u64(fieldName);
      }
      case "i64": {
        return borsh.i64(fieldName);
      }
      case "bytes": {
        return borsh.vecU8(fieldName);
      }
      case "string": {
        return borsh.str(fieldName);
      }
      case "publicKey": {
        return borsh.publicKey(fieldName);
      }
      // TODO: all the other types that need to be exported by the borsh package.
      default: {
        // @ts-ignore
        if (field.type.vec) {
          return borsh.vec(
            IdlCoder.fieldLayout(
              {
                name: undefined,
                // @ts-ignore
                type: field.type.vec,
              },
              types
            ),
            fieldName
          );
          // @ts-ignore
        } else if (field.type.option) {
          return borsh.option(
            IdlCoder.fieldLayout(
              {
                name: undefined,
                // @ts-ignore
                type: field.type.option,
              },
              types
            ),
            fieldName
          );
          // @ts-ignore
        } else if (field.type.defined) {
          // User defined type.
          if (types === undefined) {
            throw new IdlError("User defined types not provided");
          }
          // @ts-ignore
          const filtered = types.filter((t) => t.name === field.type.defined);
          if (filtered.length !== 1) {
            throw new IdlError(`Type not found: ${JSON.stringify(field)}`);
          }
          return IdlCoder.typeDefLayout(filtered[0], types, fieldName);
        } else {
          throw new Error(`Not yet implemented: ${field}`);
        }
      }
    }
  }

  public static typeDefLayout(
    typeDef: IdlTypeDef,
    types: IdlTypeDef[],
    name?: string
  ): Layout {
    if (typeDef.type.kind === "struct") {
      const fieldLayouts = typeDef.type.fields.map((field) => {
        const x = IdlCoder.fieldLayout(field, types);
        return x;
      });
      return borsh.struct(fieldLayouts, name);
    } else if (typeDef.type.kind === "enum") {
      let variants = typeDef.type.variants.map((variant: IdlEnumVariant) => {
        const name = camelCase(variant.name);
        if (variant.fields === undefined) {
          return borsh.struct([], name);
        }
        // @ts-ignore
        const fieldLayouts = variant.fields.map((f: IdlField | IdlType) => {
          // @ts-ignore
          if (f.name === undefined) {
            throw new Error("Tuple enum variants not yet implemented.");
          }
          // @ts-ignore
          return IdlCoder.fieldLayout(f, types);
        });
        return borsh.struct(fieldLayouts, name);
      });

      if (name !== undefined) {
        // Buffer-layout lib requires the name to be null (on construction)
        // when used as a field.
        return borsh.rustEnum(variants).replicate(name);
      }

      return borsh.rustEnum(variants, name);
    } else {
      throw new Error(`Unknown type kint: ${typeDef}`);
    }
  }
}
