import { Buffer } from "buffer";
import { AccountMeta, PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import {
  Idl,
  IdlField,
  IdlType,
  IdlTypeDef,
  IdlInstructionAccountItem,
  IdlDiscriminator,
  IdlArrayLen,
} from "../../idl.js";
import { InstructionCoder } from "../index.js";
import type { Instruction, InstructionDisplay } from "../borsh/instruction.js";

/**
 * Encodes Anchor v2 program instructions via the wincode wire format.
 *
 * The on-chain v2 handler reads its arg struct with
 * `wincode::deserialize(&ix_data)`, which differs from borsh on one axis:
 * length prefixes for `Vec<T>` / `&[T]` / `String` / `bytes` are **u64 LE**
 * instead of borsh's u32 LE. Fixed-size primitives, `Address`, `[T; N]`
 * arrays, `Option`'s tag byte, and enum tag bytes all encode bit-for-bit
 * identically, so the wire differs only where a length prefix shows up.
 *
 * The decode path throws — instruction-level decoding on the client is
 * rarely needed and the wincode u64 prefix doesn't fit buffer-layout's
 * abstractions cleanly. The few places that need it (log formatting,
 * inspection tools) can decode structurally via the IDL.
 */
export class WincodeInstructionCoder implements InstructionCoder {
  private ixEntries: Map<
    string,
    { discriminator: IdlDiscriminator; args: IdlField[] }
  >;

  public constructor(private idl: Idl) {
    this.ixEntries = new Map(
      idl.instructions.map((ix) => [
        ix.name,
        { discriminator: ix.discriminator, args: ix.args as IdlField[] },
      ]),
    );
  }

  public encode(ixName: string, ix: any): Buffer {
    const entry = this.ixEntries.get(ixName);
    if (!entry) {
      throw new Error(`Unknown method: ${ixName}`);
    }
    const parts: Buffer[] = [Buffer.from(entry.discriminator)];
    for (const arg of entry.args) {
      encodeType(arg.type, ix[arg.name], parts, this.idl.types ?? []);
    }
    return Buffer.concat(parts);
  }

  public decode(_ix: Buffer | string): Instruction | null {
    // Unused on the client today. The on-chain handler reads the wire via
    // `wincode::deserialize`; clients that want to inspect an
    // outgoing-or-incoming ix can walk the IDL types themselves.
    return null;
  }

  public format(
    _ix: Instruction,
    _accountMetas: AccountMeta[],
  ): InstructionDisplay | null {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Type encoders — walk the IDL type and append bytes.
// ---------------------------------------------------------------------------

function encodeType(
  ty: IdlType,
  value: any,
  out: Buffer[],
  types: IdlTypeDef[],
): void {
  if (typeof ty === "string") {
    encodePrimitive(ty, value, out);
    return;
  }
  if ("vec" in ty) {
    // u64 LE length prefix + each element. This is the one axis where
    // wincode diverges from borsh (borsh uses u32 LE here).
    writeU64Len(value.length, out);
    for (const elem of value) {
      encodeType(ty.vec, elem, out, types);
    }
    return;
  }
  if ("option" in ty) {
    if (value === null || value === undefined) {
      out.push(Buffer.from([0]));
    } else {
      out.push(Buffer.from([1]));
      encodeType(ty.option, value, out, types);
    }
    return;
  }
  if ("coption" in ty) {
    // COption is a 4-byte tag (0 / 1) for SPL-Token-style null markers.
    // Not currently used by anchor-v2 handler args; keep the same behavior
    // as borsh for future compatibility.
    const tag = Buffer.alloc(4);
    tag.writeUInt32LE(value == null ? 0 : 1, 0);
    out.push(tag);
    if (value != null) {
      encodeType(ty.coption, value, out, types);
    }
    return;
  }
  if ("array" in ty) {
    const [elemTy, len] = ty.array;
    const n = resolveArrayLen(len);
    if (!Array.isArray(value) || value.length !== n) {
      throw new Error(
        `wincode: expected array of length ${n}, got length ${
          Array.isArray(value) ? value.length : typeof value
        }`,
      );
    }
    for (const elem of value) {
      encodeType(elemTy, elem, out, types);
    }
    return;
  }
  if ("defined" in ty) {
    encodeDefined(ty.defined.name, value, out, types);
    return;
  }
  if ("generic" in ty) {
    throw new Error("wincode: generic type parameters not yet supported");
  }
  throw new Error(`wincode: unsupported IDL type: ${JSON.stringify(ty)}`);
}

function encodePrimitive(
  ty: string,
  value: any,
  out: Buffer[],
): void {
  switch (ty) {
    case "bool": {
      out.push(Buffer.from([value ? 1 : 0]));
      return;
    }
    case "u8": {
      out.push(Buffer.from([Number(value) & 0xff]));
      return;
    }
    case "i8": {
      const b = Buffer.alloc(1);
      b.writeInt8(Number(value), 0);
      out.push(b);
      return;
    }
    case "u16": {
      const b = Buffer.alloc(2);
      b.writeUInt16LE(Number(value), 0);
      out.push(b);
      return;
    }
    case "i16": {
      const b = Buffer.alloc(2);
      b.writeInt16LE(Number(value), 0);
      out.push(b);
      return;
    }
    case "u32": {
      const b = Buffer.alloc(4);
      b.writeUInt32LE(Number(value), 0);
      out.push(b);
      return;
    }
    case "i32": {
      const b = Buffer.alloc(4);
      b.writeInt32LE(Number(value), 0);
      out.push(b);
      return;
    }
    case "u64": {
      const b = Buffer.alloc(8);
      b.writeBigUInt64LE(toBigInt(value), 0);
      out.push(b);
      return;
    }
    case "i64": {
      const b = Buffer.alloc(8);
      b.writeBigInt64LE(toBigInt(value), 0);
      out.push(b);
      return;
    }
    case "u128": {
      out.push(encodeU128LE(toBigInt(value)));
      return;
    }
    case "i128": {
      out.push(encodeI128LE(toBigInt(value)));
      return;
    }
    case "f32": {
      const b = Buffer.alloc(4);
      b.writeFloatLE(Number(value), 0);
      out.push(b);
      return;
    }
    case "f64": {
      const b = Buffer.alloc(8);
      b.writeDoubleLE(Number(value), 0);
      out.push(b);
      return;
    }
    case "string": {
      const bytes = Buffer.from(String(value), "utf8");
      writeU64Len(bytes.length, out);
      out.push(bytes);
      return;
    }
    case "bytes": {
      const bytes = Buffer.isBuffer(value) ? value : Buffer.from(value);
      writeU64Len(bytes.length, out);
      out.push(bytes);
      return;
    }
    case "pubkey": {
      const pk =
        value instanceof PublicKey
          ? value
          : new PublicKey(value as Uint8Array | string);
      out.push(Buffer.from(pk.toBytes()));
      return;
    }
    default:
      throw new Error(`wincode: unknown primitive type: ${ty}`);
  }
}

function encodeDefined(
  name: string,
  value: any,
  out: Buffer[],
  types: IdlTypeDef[],
): void {
  const td = types.find((t) => t.name === name);
  if (!td) {
    throw new Error(`wincode: type '${name}' not in IDL types[]`);
  }
  switch (td.type.kind) {
    case "struct": {
      const fields = td.type.fields ?? [];
      if (Array.isArray(fields) && fields.length > 0) {
        const first = fields[0];
        if (typeof first === "object" && first !== null && "name" in first) {
          // Named struct.
          for (const f of fields as IdlField[]) {
            encodeType(f.type, value[f.name], out, types);
          }
        } else {
          // Tuple struct: positional fields.
          if (!Array.isArray(value)) {
            throw new Error(
              `wincode: tuple struct '${name}' expects array value`,
            );
          }
          for (let i = 0; i < fields.length; i++) {
            encodeType(fields[i] as IdlType, value[i], out, types);
          }
        }
      }
      return;
    }
    case "enum": {
      // Wincode enum tag byte matches borsh — 1 byte discriminant.
      const variants = td.type.variants;
      const variantName = Object.keys(value)[0];
      const idx = variants.findIndex((v) => v.name === variantName);
      if (idx < 0) {
        throw new Error(
          `wincode: enum '${name}' has no variant '${variantName}'`,
        );
      }
      out.push(Buffer.from([idx]));
      const payload = value[variantName];
      const vFields = variants[idx].fields ?? [];
      if (Array.isArray(vFields) && vFields.length > 0) {
        const first = vFields[0];
        if (typeof first === "object" && first !== null && "name" in first) {
          for (const f of vFields as IdlField[]) {
            encodeType(f.type, payload[f.name], out, types);
          }
        } else {
          if (!Array.isArray(payload)) {
            throw new Error(
              `wincode: tuple variant '${variantName}' expects array payload`,
            );
          }
          for (let i = 0; i < vFields.length; i++) {
            encodeType(vFields[i] as IdlType, payload[i], out, types);
          }
        }
      }
      return;
    }
    case "type": {
      encodeType(td.type.alias, value, out, types);
      return;
    }
  }
}

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

function writeU64Len(n: number, out: Buffer[]): void {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(n), 0);
  out.push(b);
}

function toBigInt(value: any): bigint {
  if (typeof value === "bigint") return value;
  if (value instanceof BN) return BigInt(value.toString());
  if (typeof value === "number") return BigInt(value);
  if (typeof value === "string") return BigInt(value);
  // Fallback: try BN's toString path (covers anchor.BN, which extends bn.js).
  if (value && typeof value.toString === "function") {
    return BigInt(value.toString());
  }
  throw new Error(`wincode: cannot coerce ${typeof value} to BigInt`);
}

function encodeU128LE(v: bigint): Buffer {
  // Avoid `1n << 64n` literals — tsconfig here still targets <ES2020.
  const shift64 = BigInt(64);
  const mask = (BigInt(1) << shift64) - BigInt(1);
  const b = Buffer.alloc(16);
  b.writeBigUInt64LE(v & mask, 0);
  b.writeBigUInt64LE((v >> shift64) & mask, 8);
  return b;
}

function encodeI128LE(v: bigint): Buffer {
  // Two's complement for negatives: add 2^128 then encode unsigned.
  const mod = BigInt(1) << BigInt(128);
  const u = ((v % mod) + mod) % mod;
  return encodeU128LE(u);
}

function resolveArrayLen(len: IdlArrayLen): number {
  if (typeof len === "number") return len;
  throw new Error(
    "wincode: generic array lengths not supported — bind the const param first",
  );
}

// Re-export the `InstructionCoder`'s shared types so callers can import
// from this module without reaching into the borsh one.
export type { Instruction, InstructionDisplay } from "../borsh/instruction.js";

// Unused params — keep the named imports alive for tooling that checks
// transitive type dependencies. Not referenced directly in this file.
// eslint-disable-next-line @typescript-eslint/no-unused-vars
type _TypeUse = IdlInstructionAccountItem;
