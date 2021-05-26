import { snakeCase } from "snake-case";
import { sha256 } from "js-sha256";
import { Idl, IdlField, IdlTypeDef, IdlEnumVariant, IdlType } from "../idl";
import { IdlError } from "../error";

export function accountSize(
  idl: Idl,
  idlAccount: IdlTypeDef
): number | undefined {
  if (idlAccount.type.kind === "enum") {
    let variantSizes = idlAccount.type.variants.map(
      (variant: IdlEnumVariant) => {
        if (variant.fields === undefined) {
          return 0;
        }
        return (
          variant.fields
            // @ts-ignore
            .map((f: IdlField | IdlType) => {
              // @ts-ignore
              if (f.name === undefined) {
                throw new Error("Tuple enum variants not yet implemented.");
              }
              // @ts-ignore
              return typeSize(idl, f.type);
            })
            .reduce((a: number, b: number) => a + b)
        );
      }
    );
    return Math.max(...variantSizes) + 1;
  }
  if (idlAccount.type.fields === undefined) {
    return 0;
  }
  return idlAccount.type.fields
    .map((f) => typeSize(idl, f.type))
    .reduce((a, b) => a + b);
}

// Returns the size of the type in bytes. For variable length types, just return
// 1. Users should override this value in such cases.
function typeSize(idl: Idl, ty: IdlType): number {
  switch (ty) {
    case "bool":
      return 1;
    case "u8":
      return 1;
    case "i8":
      return 1;
    case "i16":
      return 2;
    case "u16":
      return 2;
    case "u32":
      return 4;
    case "i32":
      return 4;
    case "u64":
      return 8;
    case "i64":
      return 8;
    case "u128":
      return 16;
    case "i128":
      return 16;
    case "bytes":
      return 1;
    case "string":
      return 1;
    case "publicKey":
      return 32;
    default:
      // @ts-ignore
      if (ty.vec !== undefined) {
        return 1;
      }
      // @ts-ignore
      if (ty.option !== undefined) {
        // @ts-ignore
        return 1 + typeSize(idl, ty.option);
      }
      // @ts-ignore
      if (ty.defined !== undefined) {
        // @ts-ignore
        const filtered = idl.types.filter((t) => t.name === ty.defined);
        if (filtered.length !== 1) {
          throw new IdlError(`Type not found: ${JSON.stringify(ty)}`);
        }
        let typeDef = filtered[0];

        return accountSize(idl, typeDef);
      }
      // @ts-ignore
      if (ty.array !== undefined) {
        // @ts-ignore
        let arrayTy = ty.array[0];
        // @ts-ignore
        let arraySize = ty.array[1];
        // @ts-ignore
        return typeSize(idl, arrayTy) * arraySize;
      }
      throw new Error(`Invalid type ${JSON.stringify(ty)}`);
  }
}

// Not technically sighash, since we don't include the arguments, as Rust
// doesn't allow function overloading.
export function sighash(nameSpace: string, ixName: string): Buffer {
  let name = snakeCase(ixName);
  let preimage = `${nameSpace}:${name}`;
  return Buffer.from(sha256.digest(preimage)).slice(0, 8);
}
