import { Buffer } from "buffer";
import { PublicKey } from "@solana/web3.js";
import * as borsh from "@project-serum/borsh";

export type Idl = {
  version: string;
  name: string;
  docs?: string[];
  instructions: IdlInstruction[];
  state?: IdlState;
  accounts?: IdlAccountDef[];
  types?: IdlTypeDef[];
  events?: IdlEvent[];
  errors?: IdlErrorCode[];
  constants?: IdlConstant[];
  metadata?: IdlMetadata;
};

export type IdlMetadata = any;

export type IdlConstant = {
  name: string;
  type: IdlType;
  value: string;
};

export type IdlEvent = {
  name: string;
  fields: IdlEventField[];
};

export type IdlEventField = {
  name: string;
  type: IdlType;
  index: boolean;
};

export type IdlInstruction = {
  name: string;
  docs?: string[];
  accounts: IdlAccountItem[];
  args: IdlField[];
  returns?: IdlType;
};

export type IdlState = {
  struct: IdlTypeDef;
  methods: IdlStateMethod[];
};

export type IdlStateMethod = IdlInstruction;

export type IdlAccountItem = IdlAccount | IdlAccounts;

export type IdlAccount = {
  name: string;
  isMut: boolean;
  isSigner: boolean;
  docs?: string[];
  pda?: IdlPda;
};

export type IdlPda = {
  seeds: IdlSeed[];
  programId?: IdlSeed;
};

export type IdlSeed = any; // TODO

// A nested/recursive version of IdlAccount.
export type IdlAccounts = {
  name: string;
  docs?: string[];
  accounts: IdlAccountItem[];
};

export type IdlField = {
  name: string;
  docs?: string[];
  type: IdlType;
};

export type IdlTypeDef = {
  name: string;
  docs?: string[];
  type: IdlTypeDefTy;
};

export type IdlAccountDef = {
  name: string;
  docs?: string[];
  type: IdlTypeDefTyStruct;
};

export type IdlTypeDefTyStruct = {
  kind: "struct";
  fields: IdlTypeDefStruct;
};

export type IdlTypeDefTyEnum = {
  kind: "enum";
  variants: IdlEnumVariant[];
};

export type IdlTypeDefTy = IdlTypeDefTyEnum | IdlTypeDefTyStruct;

export type IdlTypeDefStruct = Array<IdlField>;

export type IdlType =
  | "bool"
  | "u8"
  | "i8"
  | "u16"
  | "i16"
  | "u32"
  | "i32"
  | "f32"
  | "u64"
  | "i64"
  | "f64"
  | "u128"
  | "i128"
  | "bytes"
  | "string"
  | "publicKey"
  | IdlTypeDefined
  | IdlTypeOption
  | IdlTypeCOption
  | IdlTypeVec
  | IdlTypeArray;

// User defined type.
export type IdlTypeDefined = {
  defined: string;
};

export type IdlTypeOption = {
  option: IdlType;
};

export type IdlTypeCOption = {
  coption: IdlType;
};

export type IdlTypeVec = {
  vec: IdlType;
};

export type IdlTypeArray = {
  array: [idlType: IdlType, size: number];
};

export type IdlEnumVariant = {
  name: string;
  fields?: IdlEnumFields;
};

export type IdlEnumFields = IdlEnumFieldsNamed | IdlEnumFieldsTuple;

export type IdlEnumFieldsNamed = IdlField[];

export type IdlEnumFieldsTuple = IdlType[];

export type IdlErrorCode = {
  code: number;
  name: string;
  msg?: string;
};

// Deterministic IDL address as a function of the program id.
export async function idlAddress(programId: PublicKey): Promise<PublicKey> {
  const base = (await PublicKey.findProgramAddress([], programId))[0];
  return await PublicKey.createWithSeed(base, seed(), programId);
}

// Seed for generating the idlAddress.
export function seed(): string {
  return "anchor:idl";
}

// The on-chain account of the IDL.
export interface IdlProgramAccount {
  authority: PublicKey;
  data: Buffer;
}

const IDL_ACCOUNT_LAYOUT: borsh.Layout<IdlProgramAccount> = borsh.struct([
  borsh.publicKey("authority"),
  borsh.vecU8("data"),
]);

export function decodeIdlAccount(data: Buffer): IdlProgramAccount {
  return IDL_ACCOUNT_LAYOUT.decode(data);
}

export function encodeIdlAccount(acc: IdlProgramAccount): Buffer {
  const buffer = Buffer.alloc(1000); // TODO: use a tighter buffer.
  const len = IDL_ACCOUNT_LAYOUT.encode(acc, buffer);
  return buffer.slice(0, len);
}
