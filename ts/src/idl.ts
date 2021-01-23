export type Idl = {
  version: string;
  name: string;
  instructions: IdlInstruction[];
  state?: IdlState;
  accounts?: IdlTypeDef[];
  types?: IdlTypeDef[];
  errors?: IdlErrorCode[];
};

export type IdlInstruction = {
  name: string;
  accounts: IdlAccountItem[];
  args: IdlField[];
};

// IdlStateMethods are similar to instructions, except they only allow
// for a single account, the state account.
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
};

// A nested/recursive version of IdlAccount.
export type IdlAccounts = {
  name: string;
  accounts: IdlAccountItem[];
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
  variants?: IdlEnumVariant[];
};

type IdlTypeDefStruct = Array<IdlField>;

export type IdlType =
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

export type IdlEnumVariant = {
  name: string;
  fields?: IdlEnumFields;
};

type IdlEnumFields = IdlEnumFieldsNamed | IdlEnumFieldsTuple;

type IdlEnumFieldsNamed = IdlField[];

type IdlEnumFieldsTuple = IdlType[];

type IdlErrorCode = {
  code: number;
  name: string;
  msg?: string;
};
