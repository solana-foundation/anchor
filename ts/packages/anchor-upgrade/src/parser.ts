import { AccountClient, ConstantClient, EventClient, InstructionClient } from './clients';
import type {
  AccountName,
  CamelizedIdl,
  ConstantName,
  EventName,
  Idl,
  InstructionName,
} from './types';
import { convertIdlToCamelCase } from './utils';

type AccountRegistry<IDL extends Idl> = {
  [N in AccountName<IDL>]: AccountClient<IDL, N>;
};

type InstructionRegistry<IDL extends Idl> = {
  [N in InstructionName<IDL>]: InstructionClient<IDL, N>;
};

type EventRegistry<IDL extends Idl> = {
  [N in EventName<IDL>]: EventClient<IDL, N>;
};

type ConstantRegistry<IDL extends Idl> = {
  [N in ConstantName<IDL>]: ConstantClient<IDL, N>;
};

type NamedItem = { name: string };

export class IdlParser<const RawIDL extends Idl> {
  readonly idl: CamelizedIdl<RawIDL>;

  readonly accounts: AccountRegistry<CamelizedIdl<RawIDL>>;
  readonly instructions: InstructionRegistry<CamelizedIdl<RawIDL>>;
  readonly events: EventRegistry<CamelizedIdl<RawIDL>>;
  readonly constants: ConstantRegistry<CamelizedIdl<RawIDL>>;

  constructor(rawIdl: RawIDL) {
    this.idl = convertIdlToCamelCase(rawIdl);

    this.accounts = this.buildRegistry(
      this.idl.accounts,
      (df) => new AccountClient(this.idl, df as never),
    );

    this.instructions = this.buildRegistry(
      this.idl.instructions,
      (df) => new InstructionClient(this.idl, df as never),
    );

    this.events = this.buildRegistry(
      this.idl.events,
      (df) => new EventClient(this.idl, df as never),
    );

    this.constants = this.buildRegistry(
      this.idl.constants,
      (df) => new ConstantClient(this.idl, df as never),
    );
  }

  private buildRegistry<TItem extends NamedItem, TRegistry extends Record<string, unknown>>(
    items: readonly TItem[] | undefined,
    createValue: (item: TItem) => TRegistry[keyof TRegistry],
  ): TRegistry {
    const entries = (items ?? []).map((item) => [item.name, createValue(item)] as const);
    return Object.fromEntries(entries) as TRegistry;
  }
}
