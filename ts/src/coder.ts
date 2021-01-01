import { Layout } from "buffer-layout";
import * as borsh from "@project-serum/borsh";
import { Idl, IdlField, IdlTypeDef } from "./idl";
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

  constructor(idl: Idl) {
    this.instruction = new InstructionCoder(idl);
    this.accounts = new AccountsCoder(idl);
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
      let fieldLayouts = ix.args.map((arg) =>
        IdlCoder.fieldLayout(arg, idl.types)
      );
      return borsh.struct(fieldLayouts, ix.name);
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

class IdlCoder {
  public static fieldLayout(field: IdlField, types?: IdlTypeDef[]): Layout {
    switch (field.type) {
      case "bool": {
        return borsh.bool(field.name);
      }
      case "u8": {
        return borsh.u8(field.name);
      }
      case "u32": {
        return borsh.u32(field.name);
      }
      case "u64": {
        return borsh.u64(field.name);
      }
      case "i64": {
        return borsh.i64(field.name);
      }
      case "bytes": {
        return borsh.vecU8(field.name);
      }
      case "string": {
        return borsh.str(field.name);
      }
      case "publicKey": {
        return borsh.publicKey(field.name);
      }
      // TODO: all the other types that need to be exported by the borsh package.
      default: {
        // @ts-ignore
        if (field.type.option) {
          return borsh.option(
            IdlCoder.fieldLayout(
              {
                name: undefined,
                // @ts-ignore
                type: field.type.option,
              },
              types
            ),
            field.name
          );
          // @ts-ignore
        } else if (field.type.defined) {
          // User defined type.
          if (types === undefined) {
            throw new IdlError("User defined types not provided");
          }
          // @ts-ignore
          const name = field.type.defined;
          const filtered = types.filter((t) => t.name === name);
          if (filtered.length !== 1) {
            console.log(types);
            console.log(name);
            throw new IdlError("Type not found");
          }
          return IdlCoder.typeDefLayout(filtered[0], types, name);
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
      const fieldLayouts = typeDef.type.fields.map((field) =>
        IdlCoder.fieldLayout(field, types)
      );
      return borsh.struct(fieldLayouts, name);
    } else {
      // TODO: enums
      throw new Error("Enums not yet implemented");
    }
  }
}
