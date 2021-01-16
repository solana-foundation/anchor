export type Idl = {
  version: string;
  name: string;
  instructions: IdlInstruction[];
  accounts?: IdlTypeDef[];
  types?: IdlTypeDef[];
  errors?: IdlErrorCode[];
};

export type IdlInstruction = {
  name: string;
  accounts: IdlAccount[];
  args: IdlField[];
};

export type IdlAccount = {
  name: string;
  isMut: boolean;
  isSigner: boolean;
};

export type IdlField = {
  name: string;
  type: IdlType;
};

export type IdlTypeDef = {
  name: string;
  type: IdlTypeDefTy;
};

type IdlTypeDefTy = {
  kind: "struct" | "enum";
  fields?: IdlTypeDefStruct;
  variants?: IdlTypeDefEnum;
};

type IdlTypeDefStruct = Array<IdlField>;

// TODO
type IdlTypeDefEnum = {
  variants: IdlEnumVariant;
};

type IdlType =
  | "bool"
  | "u8"
  | "i8"
  | "u16"
  | "i16"
  | "u32"
  | "i32"
  | "u64"
  | "i64"
  | "bytes"
  | "string"
  | "publicKey"
  | IdlTypeVec
  | IdlTypeOption
  | IdlTypeDefined;

export type IdlTypeVec = {
  vec: IdlType;
};

export type IdlTypeOption = {
  option: IdlType;
};

// User defined type.
export type IdlTypeDefined = {
  defined: string;
};

type IdlEnumVariant = {
  // todo
};

type IdlErrorCode = {
  code: number;
  name: string;
  msg?: string;
};
