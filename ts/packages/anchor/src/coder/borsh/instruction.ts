import bs58 from "bs58";
import { Buffer } from "buffer";
import { Layout } from "buffer-layout";
import * as borsh from "@anchor-lang/borsh";
import { AccountMeta, PublicKey } from "@solana/web3.js";
import {
  handleDefinedFields,
  Idl,
  IdlField,
  IdlType,
  IdlTypeDef,
  IdlAccount,
  IdlInstructionAccountItem,
  IdlTypeVec,
  IdlInstructionAccounts,
  IdlDiscriminator,
} from "../../idl.js";
import { IdlCoder } from "./idl.js";
import { typeDivergesBetweenWincodeAndBorsh } from "./event.js";
import { InstructionCoder } from "../index.js";

/**
 * Encodes and decodes program instructions.
 */
export class BorshInstructionCoder implements InstructionCoder {
  // Instruction args layout. Maps namespaced method name to borsh encoder,
  // IDL-declared discriminator bytes, and an optional wincode-divergence
  // guard (see comment at the constructor for why).
  private ixLayouts: Map<
    string,
    {
      discriminator: IdlDiscriminator;
      layout: Layout;
      wincodeUnsupported?: string;
    }
  >;

  public constructor(private idl: Idl) {
    // Anchor v2 handlers decode their ix args via wincode, not borsh. The
    // two wire formats agree on primitives, fixed-size arrays, `Address`,
    // `Option` tag bytes, and enum tag bytes — anything whose encoding
    // doesn't touch a length prefix. They diverge on `Vec<T>` / `String` /
    // `bytes`: borsh uses a u32 LE length, wincode u64. A v2 handler fed a
    // borsh-encoded vec would misparse the prefix and then read the wrong
    // number of bytes. Detect divergent arg types up front and throw a
    // clear TS-side error at `encode()` instead of letting the tx fail
    // on-chain with an opaque wincode-deserialize error.
    //
    // Proper fix is a wincode instruction coder with u64 LE prefixes; this
    // guard is the small cousin that closes the silent-failure gap until
    // then.
    const ixLayouts = idl.instructions.map((ix) => {
      const name = ix.name;
      const fieldLayouts = ix.args.map((arg) =>
        IdlCoder.fieldLayout(arg, idl.types)
      );
      const layout = borsh.struct(fieldLayouts, name);
      let wincodeUnsupported: string | undefined;
      for (const arg of ix.args) {
        if (typeDivergesBetweenWincodeAndBorsh(arg.type, idl.types ?? [])) {
          wincodeUnsupported =
            `instruction \`${name}\` arg \`${arg.name}\` contains a ` +
            `Vec/String/bytes field whose wire format diverges between ` +
            `wincode (u64 LE length) and borsh (u32 LE length). Anchor v2 ` +
            `handlers decode via wincode, so a borsh-encoded payload would ` +
            `misparse on-chain. A wincode instruction coder is not yet ` +
            `implemented — build this instruction manually with a raw ` +
            `\`TransactionInstruction\` for now.`;
          break;
        }
      }
      return [
        name,
        { discriminator: ix.discriminator, layout, wincodeUnsupported },
      ] as const;
    });
    this.ixLayouts = new Map(ixLayouts);
  }

  /**
   * Encodes a program instruction.
   */
  public encode(ixName: string, ix: any): Buffer {
    const buffer = Buffer.alloc(1000); // TODO: use a tighter buffer.
    const encoder = this.ixLayouts.get(ixName);
    if (!encoder) {
      throw new Error(`Unknown method: ${ixName}`);
    }
    if (encoder.wincodeUnsupported) {
      throw new Error(encoder.wincodeUnsupported);
    }

    const len = encoder.layout.encode(ix, buffer);
    const data = buffer.slice(0, len);

    return Buffer.concat([Buffer.from(encoder.discriminator), data]);
  }

  /**
   * Decodes a program instruction.
   */
  public decode(
    ix: Buffer | string,
    encoding: "hex" | "base58" = "hex"
  ): Instruction | null {
    if (typeof ix === "string") {
      ix = encoding === "hex" ? Buffer.from(ix, "hex") : bs58.decode(ix);
    }

    for (const [name, layout] of this.ixLayouts) {
      const givenDisc = ix.subarray(0, layout.discriminator.length);
      const matches = givenDisc.equals(Buffer.from(layout.discriminator));
      if (matches) {
        return {
          name,
          data: layout.layout.decode(ix.subarray(givenDisc.length)),
        };
      }
    }

    return null;
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
    const idlIx = idl.instructions.find((i) => ix.name === i.name);
    if (!idlIx) {
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
      return idlType;
    }

    if ("option" in idlType) {
      return `Option<${this.formatIdlType(idlType.option)}>`;
    }
    if ("coption" in idlType) {
      return `COption<${this.formatIdlType(idlType.coption)}>`;
    }
    if ("vec" in idlType) {
      return `Vec<${this.formatIdlType(idlType.vec)}>`;
    }
    if ("array" in idlType) {
      return `Array<${idlType.array[0]}; ${idlType.array[1]}>`;
    }
    if ("defined" in idlType) {
      const name = idlType.defined.name;
      if (idlType.defined.generics) {
        const generics = idlType.defined.generics
          .map((g) => {
            switch (g.kind) {
              case "type":
                return InstructionFormatter.formatIdlType(g.type);
              case "const":
                return g.value;
            }
          })
          .join(", ");

        return `${name}<${generics}>`;
      }

      return name;
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
    if ("vec" in idlField.type) {
      return (
        "[" +
        (<Array<IdlField>>data)
          .map((d) =>
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
    if ("option" in idlField.type) {
      return data === null
        ? "null"
        : this.formatIdlData(
            { name: "", type: idlField.type.option },
            data,
            types
          );
    }
    if ("defined" in idlField.type) {
      if (!types) {
        throw new Error("User defined types not provided");
      }

      const definedName = idlField.type.defined.name;
      const typeDef = types.find((t) => t.name === definedName);
      if (!typeDef) {
        throw new Error(`Type not found: ${definedName}`);
      }

      return InstructionFormatter.formatIdlDataDefined(typeDef, data, types);
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
        return (
          "{ " +
          handleDefinedFields(
            typeDef.type.fields,
            () => "",
            (fields) => {
              return Object.entries(data)
                .map(([key, val]) => {
                  const field = fields.find((f) => f.name === key);
                  if (!field) {
                    throw new Error(`Field not found: ${key}`);
                  }
                  return (
                    key +
                    ": " +
                    InstructionFormatter.formatIdlData(field, val, types)
                  );
                })
                .join(", ");
            },
            (fields) => {
              return Object.entries(data)
                .map(([key, val]) => {
                  return (
                    key +
                    ": " +
                    InstructionFormatter.formatIdlData(
                      { name: "", type: fields[key] },
                      val,
                      types
                    )
                  );
                })
                .join(", ");
            }
          ) +
          " }"
        );
      }

      case "enum": {
        const variantName = Object.keys(data)[0];
        const variant = typeDef.type.variants.find(
          (v) => v.name === variantName
        );
        if (!variant) {
          throw new Error(`Unable to find variant: ${variantName}`);
        }

        const enumValue = data[variantName];
        return handleDefinedFields(
          variant.fields,
          () => variantName,
          (fields) => {
            const namedFields = Object.keys(enumValue)
              .map((f) => {
                const fieldData = enumValue[f];
                const idlField = fields.find((v) => v.name === f);
                if (!idlField) {
                  throw new Error(`Field not found: ${f}`);
                }

                return (
                  f +
                  ": " +
                  InstructionFormatter.formatIdlData(idlField, fieldData, types)
                );
              })
              .join(", ");

            return `${variantName} { ${namedFields} }`;
          },
          (fields) => {
            const tupleFields = Object.entries(enumValue)
              .map(([key, val]) => {
                return (
                  key +
                  ": " +
                  InstructionFormatter.formatIdlData(
                    { name: "", type: fields[key] },
                    val as any,
                    types
                  )
                );
              })
              .join(", ");

            return `${variantName} { ${tupleFields} }`;
          }
        );
      }

      case "type": {
        return InstructionFormatter.formatIdlType(typeDef.type.alias);
      }
    }
  }

  private static flattenIdlAccounts(
    accounts: IdlInstructionAccountItem[],
    prefix?: string
  ): IdlAccount[] {
    return accounts
      .map((account) => {
        const accName = sentenceCase(account.name);
        if (account.hasOwnProperty("accounts")) {
          const newPrefix = prefix ? `${prefix} > ${accName}` : accName;
          return InstructionFormatter.flattenIdlAccounts(
            (<IdlInstructionAccounts>account).accounts,
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
