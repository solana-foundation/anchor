import type { Idl, IdlInstructionAccountItem, IdlInstructionAccounts, IdlTypeDef } from './idl';
import type {
  AllAccounts,
  AllEvents,
  AllInstructions,
  DecodeType,
  IdlTypes,
  NullableIdlAccount,
  NullableIdlConst,
  NullableIdlEvent,
  TypeDef,
} from './type-def';

type ByName<T extends { name: string }> = {
  [K in T['name']]: Extract<T, { name: K }>;
};

export type AccountName<IDL extends Idl> = NullableIdlAccount<IDL>['name'];
export type EventName<IDL extends Idl> = NullableIdlEvent<IDL>['name'];
export type InstructionName<IDL extends Idl> = AllInstructions<IDL>['name'];
export type ConstantName<IDL extends Idl> = NullableIdlConst<IDL>['name'];

export type AccountByName<IDL extends Idl, N extends AccountName<IDL>> = Extract<
  NullableIdlAccount<IDL>,
  { name: N }
>;

export type EventByName<IDL extends Idl, N extends EventName<IDL>> = Extract<
  NullableIdlEvent<IDL>,
  { name: N }
>;

export type InstructionByName<IDL extends Idl, N extends InstructionName<IDL>> = Extract<
  AllInstructions<IDL>,
  { name: N }
>;

export type AllAccountsMap<I extends Idl> = ByName<NullableIdlAccount<I>>;
export type AllEventsMap<I extends Idl> = ByName<NullableIdlEvent<I>>;
export type AllConstantsMap<I extends Idl> = ByName<NullableIdlConst<I>>;
export type AllInstructionsMap<I extends Idl> = ByName<AllInstructions<I>>;

type DecodeNamedTypeDef<T extends IdlTypeDef[], N extends string, Defined> = TypeDef<
  Extract<T[number], { name: N }>,
  Defined
>;

export type DecodedAccount<IDL extends Idl, N extends AccountName<IDL>> = DecodeNamedTypeDef<
  AllAccounts<IDL>,
  N,
  IdlTypes<IDL>
>;

export type DecodedEvent<IDL extends Idl, N extends EventName<IDL>> = DecodeNamedTypeDef<
  AllEvents<IDL>,
  N,
  IdlTypes<IDL>
>;

export type InstructionArgs<IDL extends Idl, N extends InstructionName<IDL>> = {
  [F in InstructionByName<IDL, N>['args'][number] as F['name']]: DecodeType<
    F['type'],
    IdlTypes<IDL>
  >;
};

type FlattenInstructionAccountItem<A extends IdlInstructionAccountItem> =
  A extends IdlInstructionAccounts ? FlattenInstructionAccountItem<A['accounts'][number]> : A;

export type InstructionAccountName<
  IDL extends Idl,
  N extends InstructionName<IDL>,
> = FlattenInstructionAccountItem<InstructionByName<IDL, N>['accounts'][number]>['name'];

export type ConstantByName<IDL extends Idl, N extends ConstantName<IDL>> = Extract<
  NullableIdlConst<IDL>,
  { name: N }
>;

export type DecodedConstant<IDL extends Idl, N extends ConstantName<IDL>> = DecodeType<
  ConstantByName<IDL, N>['type'],
  IdlTypes<IDL>
>;
