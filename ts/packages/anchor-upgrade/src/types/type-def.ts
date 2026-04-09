import type { Address } from '@solana/kit';

import type {
  Idl,
  IdlAccount,
  IdlArrayLen,
  IdlConst,
  IdlDefinedFields,
  IdlDefinedFieldsNamed,
  IdlDefinedFieldsTuple,
  IdlEvent,
  IdlType,
  IdlTypeDef,
  IdlTypeDefTyEnum,
  IdlTypeDefTyStruct,
  IdlTypeDefTyType,
} from './idl';

type IdlPointerSection = keyof Pick<Idl, 'accounts' | 'events'>;

type FilterTuple<T extends unknown[], F> = T extends [infer Head, ...infer Tail]
  ? [Head] extends [F]
    ? [Head, ...FilterTuple<Tail, F>]
    : FilterTuple<Tail, F>
  : [];

type ResolveIdlTypePointer<I extends Idl, Key extends IdlPointerSection> = FilterTuple<
  NonNullable<I['types']>,
  { name: NonNullable<I[Key]>[number]['name'] }
>;

type UnboxToUnion<T> = T extends (infer U)[]
  ? UnboxToUnion<U>
  : T extends Record<string, never> // empty object, eg: named enum variant without fields
    ? '__empty_object__'
    : T extends Record<string, infer V> // object with props, eg: struct
      ? UnboxToUnion<V>
      : T;

type DecodedHelper<T extends IdlTypeDef[], Defined> = {
  [D in T[number] as D['name']]: TypeDef<D, Defined>;
};

type UnknownType = '__unknown_defined_type__';

type EmptyDefined = Record<UnknownType, never>;

type RecursiveDepth2<
  T extends IdlTypeDef[],
  Defined = EmptyDefined,
  Decoded = DecodedHelper<T, Defined>,
> = UnknownType extends UnboxToUnion<Decoded>
  ? RecursiveDepth3<T, DecodedHelper<T, Defined>>
  : Decoded;

type RecursiveDepth3<
  T extends IdlTypeDef[],
  Defined = EmptyDefined,
  Decoded = DecodedHelper<T, Defined>,
> = UnknownType extends UnboxToUnion<Decoded>
  ? RecursiveDepth4<T, DecodedHelper<T, Defined>>
  : Decoded;

type RecursiveDepth4<T extends IdlTypeDef[], Defined = EmptyDefined> = DecodedHelper<T, Defined>;

type RecursiveTypes<
  T extends IdlTypeDef[],
  Defined = EmptyDefined,
  Decoded = DecodedHelper<T, Defined>,
> = UnknownType extends UnboxToUnion<Decoded>
  ? RecursiveDepth2<T, DecodedHelper<T, Defined>>
  : Decoded;

export type IdlTypes<I extends Idl> = RecursiveTypes<NonNullable<I['types']>>;

type TypeMap = {
  pubkey: Address<string>;
  bool: boolean;
  string: string;
  bytes: Buffer;
} & {
  [K in 'u8' | 'i8' | 'u16' | 'i16' | 'u32' | 'i32' | 'f32' | 'f64']: number;
} & {
  [K in 'u64' | 'i64' | 'u128' | 'i128' | 'u256' | 'i256']: bigint;
};

export type DecodeType<T extends IdlType, Defined> = IdlType extends T
  ? unknown
  : T extends keyof TypeMap
    ? TypeMap[T]
    : T extends { defined: { name: keyof Defined } }
      ? Defined[T['defined']['name']]
      : T extends { option: IdlType }
        ? DecodeType<T['option'], Defined> | null
        : T extends { coption: IdlType }
          ? DecodeType<T['coption'], Defined> | null
          : T extends { vec: IdlType }
            ? DecodeType<T['vec'], Defined>[]
            : T extends { array: [defined: IdlType, size: IdlArrayLen] }
              ? DecodeType<T['array'][0], Defined>[]
              : unknown;

type DecodeDefinedField<F, Defined> = F extends IdlType ? DecodeType<F, Defined> : never;

type DecodeDefinedFields<F extends IdlDefinedFields, Defined> = F extends IdlDefinedFieldsNamed
  ? {
      [F2 in F[number] as F2['name']]: DecodeDefinedField<F2['type'], Defined>;
    }
  : F extends IdlDefinedFieldsTuple
    ? {
        [F3 in keyof F as Exclude<F3, keyof unknown[]>]: DecodeDefinedField<F[F3], Defined>;
      }
    : Record<string, never>;

type DecodeEnumVariants<I extends IdlTypeDefTyEnum, Defined> = {
  [V in I['variants'][number] as V['name']]: DecodeDefinedFields<NonNullable<V['fields']>, Defined>;
};

type ValueOf<T> = T[keyof T];
type XorEnumVariants<T extends Record<string, unknown>> = ValueOf<{
  [K1 in keyof T]: {
    [K2 in Exclude<keyof T, K1>]?: never;
  } & { [K2 in K1]: T[K2] };
}>;

type DecodeEnum<I extends IdlTypeDefTyEnum, Defined> = XorEnumVariants<
  DecodeEnumVariants<I, Defined>
>;

type DecodeStruct<I extends IdlTypeDefTyStruct, Defined> = DecodeDefinedFields<
  NonNullable<I['fields']>,
  Defined
>;

type DecodeAlias<I extends IdlTypeDefTyType, Defined> = DecodeType<I['alias'], Defined>;

export type TypeDef<I extends IdlTypeDef, Defined> = I['type'] extends IdlTypeDefTyEnum
  ? DecodeEnum<I['type'], Defined>
  : I['type'] extends IdlTypeDefTyStruct
    ? DecodeStruct<I['type'], Defined>
    : I['type'] extends IdlTypeDefTyType
      ? DecodeAlias<I['type'], Defined>
      : never;

export type NullableIdlAccount<IDL extends Idl> = IDL['accounts'] extends undefined
  ? IdlAccount
  : NonNullable<IDL['accounts']>[number];

export type NullableIdlEvent<IDL extends Idl> = IDL['events'] extends undefined
  ? IdlEvent
  : NonNullable<IDL['events']>[number];

export type NullableIdlConst<IDL extends Idl> = IDL['constants'] extends undefined
  ? IdlConst
  : NonNullable<IDL['constants']>[number];

export type AllEvents<I extends Idl> = ResolveIdlTypePointer<I, 'events'>;
export type AllAccounts<I extends Idl> = ResolveIdlTypePointer<I, 'accounts'>;
export type AllInstructions<I extends Idl> = I['instructions'][number];
