export type Idl = {
  address: string;
  metadata: IdlMetadata;
  docs?: string[];
  instructions: IdlInstruction[];
  accounts?: IdlAccount[];
  events?: IdlEvent[];
  errors?: IdlErrorCode[];
  types?: IdlTypeDef[];
  constants?: IdlConst[];
};

export type IdlMetadata = {
  name: string;
  version: string;
  spec: string;
  description?: string;
  repository?: string;
  dependencies?: IdlDependency[];
  contact?: string;
  deployments?: IdlDeployments;
};

export type IdlDependency = {
  name: string;
  version: string;
};

export type IdlDeployments = {
  mainnet?: string;
  testnet?: string;
  devnet?: string;
  localnet?: string;
};

export type IdlInstruction = {
  name: string;
  docs?: string[];
  discriminator: IdlDiscriminator;
  accounts: IdlInstructionAccountItem[];
  args: IdlField[];
  returns?: IdlType;
};

export type IdlInstructionAccountItem = IdlInstructionAccount | IdlInstructionAccounts;

export type IdlInstructionAccount = {
  name: string;
  docs?: string[];
  writable?: boolean;
  signer?: boolean;
  optional?: boolean;
  address?: string;
  pda?: IdlPda;
  relations?: string[];
};

export type IdlInstructionAccounts = {
  name: string;
  accounts: IdlInstructionAccount[];
};

export type IdlPda = {
  seeds: IdlSeed[];
  program?: IdlSeed;
};

export type IdlSeed = IdlSeedConst | IdlSeedArg | IdlSeedAccount;

export type IdlSeedConst = {
  kind: 'const';
  value: number[];
};

export type IdlSeedArg = {
  kind: 'arg';
  path: string;
};

export type IdlSeedAccount = {
  kind: 'account';
  path: string;
  account?: string;
};

export type IdlAccount = {
  name: string;
  discriminator: IdlDiscriminator;
};

export type IdlEvent = {
  name: string;
  discriminator: IdlDiscriminator;
};

export type IdlConst = {
  name: string;
  type: IdlType;
  value: string;
};

export type IdlErrorCode = {
  name: string;
  code: number;
  msg?: string;
};

export type IdlField = {
  name: string;
  docs?: string[];
  type: IdlType;
};

export type IdlTypeDef = {
  name: string;
  docs?: string[];
  serialization?: IdlSerialization;
  repr?: IdlRepr;
  generics?: IdlTypeDefGeneric[];
  type: IdlTypeDefTy;
};

export type IdlSerialization = 'borsh' | 'bytemuck' | 'bytemuckunsafe' | { custom: string };

export type IdlRepr = IdlReprRust | IdlReprC | IdlReprTransparent;

export type IdlReprRust = {
  kind: 'rust';
} & IdlReprModifier;

export type IdlReprC = {
  kind: 'c';
} & IdlReprModifier;

export type IdlReprTransparent = {
  kind: 'transparent';
};

export type IdlReprModifier = {
  packed?: boolean;
  align?: number;
};

export type IdlTypeDefGeneric = IdlTypeDefGenericType | IdlTypeDefGenericConst;

export type IdlTypeDefGenericType = {
  kind: 'type';
  name: string;
};

export type IdlTypeDefGenericConst = {
  kind: 'const';
  name: string;
  type: string;
};

export type IdlTypeDefTy = IdlTypeDefTyEnum | IdlTypeDefTyStruct | IdlTypeDefTyType;

export type IdlTypeDefTyStruct = {
  kind: 'struct';
  fields?: IdlDefinedFields;
};

export type IdlTypeDefTyEnum = {
  kind: 'enum';
  variants: IdlEnumVariant[];
};

export type IdlTypeDefTyType = {
  kind: 'type';
  alias: IdlType;
};

export type IdlEnumVariant = {
  name: string;
  fields?: IdlDefinedFields;
};

export type IdlDefinedFields = IdlDefinedFieldsNamed | IdlDefinedFieldsTuple;

export type IdlDefinedFieldsNamed = IdlField[];

export type IdlDefinedFieldsTuple = IdlType[];

export type IdlArrayLen = IdlArrayLenGeneric | IdlArrayLenValue;

export type IdlArrayLenGeneric = {
  generic: string;
};

export type IdlArrayLenValue = number;

export type IdlGenericArg = IdlGenericArgType | IdlGenericArgConst;

export type IdlGenericArgType = { kind: 'type'; type: IdlType };

export type IdlGenericArgConst = { kind: 'const'; value: string };

export type IdlType =
  | 'bool'
  | 'u8'
  | 'i8'
  | 'u16'
  | 'i16'
  | 'u32'
  | 'i32'
  | 'f32'
  | 'u64'
  | 'i64'
  | 'f64'
  | 'u128'
  | 'i128'
  | 'u256'
  | 'i256'
  | 'bytes'
  | 'string'
  | 'pubkey'
  | IdlTypeOption
  | IdlTypeCOption
  | IdlTypeVec
  | IdlTypeArray
  | IdlTypeDefined
  | IdlTypeGeneric;

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
  array: [idlType: IdlType, size: IdlArrayLen];
};

export type IdlTypeDefined = {
  defined: {
    name: string;
    generics?: IdlGenericArg[];
  };
};

export type IdlTypeGeneric = {
  generic: string;
};

export type IdlDiscriminator = number[];
