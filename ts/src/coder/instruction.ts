import camelCase from "camelcase";
import { Layout } from "buffer-layout";
import * as borsh from "@project-serum/borsh";
import * as bs58 from "bs58";
import {
  Idl,
  IdlField,
  IdlStateMethod,
  IdlType,
  IdlTypeDef,
  IdlAccount,
  IdlAccountItem,
  IdlTypeDefTyStruct,
} from "../idl";
import { IdlCoder } from "./idl";
import { sighash } from "./common";
import { AccountMeta, PublicKey } from "@solana/web3.js";

/**
 * Namespace for state method function signatures.
 */
export const SIGHASH_STATE_NAMESPACE = "state";
/**
 * Namespace for global instruction function signatures (i.e. functions
 * that aren't namespaced by the state or any of its trait implementations).
 */
export const SIGHASH_GLOBAL_NAMESPACE = "global";

/**
 * Encodes and decodes program instructions.
 */
export class InstructionCoder {
  // Instruction args layout. Maps namespaced method
  private ixLayout: Map<string, Layout>;

  // Base58 encoded sighash to instruction layout.
  private sighashLayouts: Map<string, { layout: Layout; name: string }>;

  public constructor(private idl: Idl) {
    this.ixLayout = InstructionCoder.parseIxLayout(idl);

    const sighashLayouts = new Map();
    idl.instructions.forEach((ix) => {
      const sh = sighash(SIGHASH_GLOBAL_NAMESPACE, ix.name);
      sighashLayouts.set(bs58.encode(sh), {
        layout: this.ixLayout.get(ix.name),
        name: ix.name,
      });
    });

    if (idl.state) {
      idl.state.methods.map((ix) => {
        const sh = sighash(SIGHASH_STATE_NAMESPACE, ix.name);
        sighashLayouts.set(bs58.encode(sh), {
          layout: this.ixLayout.get(ix.name) as Layout,
          name: ix.name,
        });
      });
    }

    this.sighashLayouts = sighashLayouts;
  }

  /**
   * Encodes a program instruction.
   */
  public encode(ixName: string, ix: any) {
    return this._encode(SIGHASH_GLOBAL_NAMESPACE, ixName, ix);
  }

  /**
   * Encodes a program state instruction.
   */
  public encodeState(ixName: string, ix: any) {
    return this._encode(SIGHASH_STATE_NAMESPACE, ixName, ix);
  }

  private _encode(nameSpace: string, ixName: string, ix: any): Buffer {
    const buffer = Buffer.alloc(1000); // TODO: use a tighter buffer.
    const methodName = camelCase(ixName);
    const layout = this.ixLayout.get(methodName);
    if (!layout) {
      throw new Error(`Unknown method: ${methodName}`);
    }
    const len = layout.encode(ix, buffer);
    const data = buffer.slice(0, len);
    return Buffer.concat([sighash(nameSpace, ixName), data]);
  }

  private static parseIxLayout(idl: Idl): Map<string, Layout> {
    const stateMethods = idl.state ? idl.state.methods : [];

    const ixLayouts = stateMethods
      .map((m: IdlStateMethod) => {
        let fieldLayouts = m.args.map((arg: IdlField) => {
          return IdlCoder.fieldLayout(
            arg,
            Array.from([...(idl.accounts ?? []), ...(idl.types ?? [])])
          );
        });
        const name = camelCase(m.name);
        return [name, borsh.struct(fieldLayouts, name)];
      })
      .concat(
        idl.instructions.map((ix) => {
          let fieldLayouts = ix.args.map((arg: IdlField) =>
            IdlCoder.fieldLayout(
              arg,
              Array.from([...(idl.accounts ?? []), ...(idl.types ?? [])])
            )
          );
          const name = camelCase(ix.name);
          return [name, borsh.struct(fieldLayouts, name)];
        })
      );
    // @ts-ignore
    return new Map(ixLayouts);
  }

  /**
   * Dewcodes a program instruction.
   */
  public decode(
    ix: Buffer | string,
    encoding: "hex" | "base58" = "hex"
  ): Instruction | null {
    if (typeof ix === "string") {
      ix = encoding === "hex" ? Buffer.from(ix, "hex") : bs58.decode(ix);
    }
    let sighash = bs58.encode(ix.slice(0, 8));
    let data = ix.slice(8);
    const decoder = this.sighashLayouts.get(sighash);
    if (!decoder) {
      return null;
    }
    return {
      data: decoder.layout.decode(data),
      name: decoder.name,
    };
  }

  /**
   * Returns a formatted table of all the fields in the given instruction data.
   */
  public format(
    ix: Instruction,
    accountMetas: AccountMeta[]
  ): InstructionDisplay | null {
    return InstructionFormatter.format(ix, accountMetas, this.idl);
  }
}

export type Instruction = {
  name: string;
  data: Object;
};

export type InstructionDisplay = {
  args: { name: string; type: string; data: string }[];
  accounts: {
    name?: string;
    pubkey: PublicKey;
    isSigner: boolean;
    isWritable: boolean;
  }[];
};

class InstructionFormatter {
  public static format(
    ix: Instruction,
    accountMetas: AccountMeta[],
    idl: Idl
  ): InstructionDisplay | null {
    const idlIx = idl.instructions.filter((i) => ix.name === i.name)[0];
    if (idlIx === undefined) {
      console.error("Invalid instruction given");
      return null;
    }

    const args = idlIx.args.map((idlField) => {
      return {
        name: idlField.name,
        type: InstructionFormatter.formatIdlType(idlField.type),
        data: InstructionFormatter.formatIdlData(
          idlField,
          ix.data[idlField.name],
          idl.types
        ),
      };
    });

    const flatIdlAccounts = InstructionFormatter.flattenIdlAccounts(
      idlIx.accounts
    );

    const accounts = accountMetas.map((meta, idx) => {
      if (idx < flatIdlAccounts.length) {
        return {
          name: flatIdlAccounts[idx].name,
          ...meta,
        };
      }
      // "Remaining accounts" are unnamed in Anchor.
      else {
        return {
          name: undefined,
          ...meta,
        };
      }
    });

    return {
      args,
      accounts,
    };
  }

  private static formatIdlType(idlType: IdlType): string {
    if (typeof idlType === "string") {
      return idlType as string;
    }

    if ("vec" in idlType) {
      return `Vec<${this.formatIdlType(idlType.vec)}>`;
    }
    if ("option" in idlType) {
      return `Option<${this.formatIdlType(idlType.option)}>`;
    }
    if ("defined" in idlType) {
      return idlType.defined;
    }
    if ("array" in idlType) {
      return `Array<${idlType.array[0]}; ${idlType.array[1]}>`;
    }

    throw new Error(`Unknown IDL type: ${idlType}`);
  }

  private static formatIdlData(
    idlField: IdlField,
    data: Object,
    types?: IdlTypeDef[]
  ): string {
    if (typeof idlField.type === "string") {
      return data.toString();
    }
    // @ts-ignore
    if (idlField.type.vec) {
      // @ts-ignore
      return (
        "[" +
        data
          // @ts-ignore
          .map((d: IdlField) =>
            this.formatIdlData(
              // @ts-ignore
              { name: "", type: idlField.type.vec },
              d
            )
          )
          .join(", ") +
        "]"
      );
    }
    // @ts-ignore
    if (idlField.type.option) {
      // @ts-ignore
      return data === null
        ? "null"
        : this.formatIdlData(
            // @ts-ignore
            { name: "", type: idlField.type.option },
            data
          );
    }
    // @ts-ignore
    if (idlField.type.defined) {
      if (types === undefined) {
        throw new Error("User defined types not provided");
      }
      // @ts-ignore
      const filtered = types.filter((t) => t.name === idlField.type.defined);
      if (filtered.length !== 1) {
        // @ts-ignore
        throw new Error(`Type not found: ${idlField.type.defined}`);
      }
      return InstructionFormatter.formatIdlDataDefined(
        filtered[0],
        data,
        types
      );
    }

    return "unknown";
  }

  private static formatIdlDataDefined(
    typeDef: IdlTypeDef,
    data: Object,
    types: IdlTypeDef[]
  ): string {
    if (typeDef.type.kind === "struct") {
      const struct: IdlTypeDefTyStruct = typeDef.type;
      const fields = Object.keys(data)
        .map((k) => {
          const f = struct.fields.filter((f) => f.name === k)[0];
          if (f === undefined) {
            throw new Error("Unable to find type");
          }
          return (
            k + ": " + InstructionFormatter.formatIdlData(f, data[k], types)
          );
        })
        .join(", ");
      return "{ " + fields + " }";
    } else {
      if (typeDef.type.variants.length === 0) {
        return "{}";
      }
      // Struct enum.
      if (typeDef.type.variants[0].name) {
        const variants = typeDef.type.variants;
        const variant = Object.keys(data)[0];
        const enumType = data[variant];
        const namedFields = Object.keys(enumType)
          .map((f) => {
            const fieldData = enumType[f];
            const idlField = variants[variant]?.filter(
              (v: IdlField) => v.name === f
            )[0];
            if (idlField === undefined) {
              throw new Error("Unable to find variant");
            }
            return (
              f +
              ": " +
              InstructionFormatter.formatIdlData(idlField, fieldData, types)
            );
          })
          .join(", ");

        const variantName = camelCase(variant, { pascalCase: true });
        if (namedFields.length === 0) {
          return variantName;
        }
        return `${variantName} { ${namedFields} }`;
      }
      // Tuple enum.
      else {
        // TODO.
        return "Tuple formatting not yet implemented";
      }
    }
  }

  private static flattenIdlAccounts(
    accounts: IdlAccountItem[],
    prefix?: string
  ): IdlAccount[] {
    // @ts-ignore
    return accounts
      .map((account) => {
        const accName = sentenceCase(account.name);
        // @ts-ignore
        if (account.accounts) {
          const newPrefix = prefix ? `${prefix} > ${accName}` : accName;
          // @ts-ignore
          return InstructionFormatter.flattenIdlAccounts(
            // @ts-ignore
            account.accounts,
            newPrefix
          );
        } else {
          return {
            ...account,
            name: prefix ? `${prefix} > ${accName}` : accName,
          };
        }
      })
      .flat();
  }
}

function sentenceCase(field: string): string {
  const result = field.replace(/([A-Z])/g, " $1");
  return result.charAt(0).toUpperCase() + result.slice(1);
}
