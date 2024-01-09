import bs58 from "bs58";
import { Buffer } from "buffer";
import { Layout } from "buffer-layout";
import camelCase from "camelcase";
import { snakeCase } from "snake-case";
import * as borsh from "@coral-xyz/borsh";
import { AccountMeta, PublicKey } from "@solana/web3.js";
import {
  Idl,
  IdlField,
  IdlType,
  IdlTypeDef,
  IdlAccount,
  IdlAccountItem,
  IdlTypeDefTyStruct,
  IdlTypeVec,
  IdlTypeOption,
  IdlTypeDefined,
  IdlAccounts,
  IdlEnumFieldsNamed,
} from "../../idl.js";
import { IdlCoder } from "./idl.js";
import { InstructionCoder } from "../index.js";
import { sha256 } from "@noble/hashes/sha256";

/**
 * Namespace for global instruction function signatures (i.e. functions
 * that aren't namespaced by the state or any of its trait implementations).
 */
export const SIGHASH_GLOBAL_NAMESPACE = "global";

/**
 * Encodes and decodes program instructions.
 */
export class BorshInstructionCoder implements InstructionCoder {
  // Instruction args layout. Maps namespaced method
  private ixLayout: Map<string, Layout>;

  // Base58 encoded sighash to instruction layout.
  private sighashLayouts: Map<string, { layout: Layout; name: string }>;

  public constructor(private idl: Idl) {
    this.ixLayout = BorshInstructionCoder.parseIxLayout(idl);

    const sighashLayouts = new Map();
    idl.instructions.forEach((ix) => {
      const sh = sighash(SIGHASH_GLOBAL_NAMESPACE, ix.name);
      sighashLayouts.set(bs58.encode(sh), {
        layout: this.ixLayout.get(ix.name),
        name: ix.name,
      });
    });

    this.sighashLayouts = sighashLayouts;
  }

  /**
   * Encodes a program instruction.
   */
  public encode(ixName: string, ix: any, discriminator?: Buffer): Buffer {
    return this._encode(
      ixName,
      ix,
      discriminator ?? sighash(SIGHASH_GLOBAL_NAMESPACE, ixName)
    );
  }

  private _encode(ixName: string, ix: any, discriminator: Buffer): Buffer {
    const buffer = Buffer.alloc(1000); // TODO: use a tighter buffer.
    const methodName = camelCase(ixName);
    const layout = this.ixLayout.get(methodName);
    if (!layout) {
      throw new Error(`Unknown method: ${methodName}`);
    }
    const len = layout.encode(ix, buffer);
    const data = buffer.slice(0, len);
    return Buffer.concat([discriminator, data]);
  }

  private static parseIxLayout(idl: Idl): Map<string, Layout> {
    const ixLayouts = idl.instructions.map((ix): [string, Layout<unknown>] => {
      let fieldLayouts = ix.args.map((arg: IdlField) =>
        IdlCoder.fieldLayout(
          arg,
          Array.from([...(idl.accounts ?? []), ...(idl.types ?? [])])
        )
      );
      const name = camelCase(ix.name);
      return [name, borsh.struct(fieldLayouts, name)];
    });

    return new Map(ixLayouts);
  }

  /**
   * Decodes a program instruction.
   */
  public decode(
    ix: Buffer | string,
    encoding: "hex" | "base58" = "hex",
    ixName?: string
  ): Instruction | null {
    if (typeof ix === "string") {
      ix = encoding === "hex" ? Buffer.from(ix, "hex") : bs58.decode(ix);
    }
    // Use the provided method name to get the sighash, ignoring the
    // discriminator in the instruction data.
    // This is useful for decoding instructions that have been encoded with a
    // different namespace, such as an SPL interface.
    let sighashKey = bs58.encode(
      ixName ? sighash(SIGHASH_GLOBAL_NAMESPACE, ixName) : ix.slice(0, 8)
    );
    let data = ix.slice(8);
    const decoder = this.sighashLayouts.get(sighashKey);
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
    if (idlField.type.hasOwnProperty("vec")) {
      return (
        "[" +
        (<Array<IdlField>>data)
          .map((d: IdlField) =>
            this.formatIdlData(
              { name: "", type: (<IdlTypeVec>idlField.type).vec },
              d,
              types
            )
          )
          .join(", ") +
        "]"
      );
    }
    if (idlField.type.hasOwnProperty("option")) {
      return data === null
        ? "null"
        : this.formatIdlData(
            { name: "", type: (<IdlTypeOption>idlField.type).option },
            data,
            types
          );
    }
    if (idlField.type.hasOwnProperty("defined")) {
      if (types === undefined) {
        throw new Error("User defined types not provided");
      }
      const filtered = types.filter(
        (t) => t.name === (<IdlTypeDefined>idlField.type).defined
      );
      if (filtered.length !== 1) {
        throw new Error(
          `Type not found: ${(<IdlTypeDefined>idlField.type).defined}`
        );
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
    switch (typeDef.type.kind) {
      case "struct": {
        const struct: IdlTypeDefTyStruct = typeDef.type;
        const fields = Object.keys(data)
          .map((k) => {
            const field = struct.fields.find((f) => f.name === k);
            if (!field) {
              throw new Error("Unable to find type");
            }
            return (
              k +
              ": " +
              InstructionFormatter.formatIdlData(field, data[k], types)
            );
          })
          .join(", ");
        return "{ " + fields + " }";
      }

      case "enum": {
        if (typeDef.type.variants.length === 0) {
          return "{}";
        }
        // Struct enum.
        if (typeDef.type.variants[0].name) {
          const variants = typeDef.type.variants;
          const variant = Object.keys(data)[0];
          const enumType = data[variant];
          const enumVariant = variants.find(
            (v) => camelCase(v.name) === variant
          );
          if (!enumVariant) {
            throw new Error(`Unable to find variant \`${variant}\``);
          }
          const fields = enumVariant.fields as IdlEnumFieldsNamed;
          const namedFields = Object.keys(enumType)
            .map((f) => {
              const fieldData = enumType[f];
              const idlField = fields.find((v) => v.name === f);
              if (!idlField) {
                throw new Error(`Unable to find field \`${f}\``);
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

      case "alias": {
        return InstructionFormatter.formatIdlType(typeDef.type.value);
      }
    }
  }

  private static flattenIdlAccounts(
    accounts: IdlAccountItem[],
    prefix?: string
  ): IdlAccount[] {
    return accounts
      .map((account) => {
        const accName = sentenceCase(account.name);
        if (account.hasOwnProperty("accounts")) {
          const newPrefix = prefix ? `${prefix} > ${accName}` : accName;
          return InstructionFormatter.flattenIdlAccounts(
            (<IdlAccounts>account).accounts,
            newPrefix
          );
        } else {
          return {
            ...(<IdlAccount>account),
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

// Not technically sighash, since we don't include the arguments, as Rust
// doesn't allow function overloading.
function sighash(nameSpace: string, ixName: string): Buffer {
  let name = snakeCase(ixName);
  let preimage = `${nameSpace}:${name}`;
  return Buffer.from(sha256(preimage).slice(0, 8));
}
