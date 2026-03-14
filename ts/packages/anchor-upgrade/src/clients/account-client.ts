import { AccountCodec } from '../codec/account-codec';
import type { AccountByName, AccountName, DecodedAccount, Idl } from '../types';

export class AccountClient<IDL extends Idl, N extends AccountName<IDL>> {
  private readonly codec: AccountCodec<DecodedAccount<IDL, N>>;

  constructor(
    private readonly idl: IDL,
    private readonly accountDef: AccountByName<IDL, N>,
  ) {
    this.codec = new AccountCodec<DecodedAccount<IDL, N>>(this.idl, this.accountDef);
  }

  get discriminator(): Uint8Array {
    return Uint8Array.from(this.accountDef.discriminator);
  }

  encode(data: DecodedAccount<IDL, N>): Uint8Array {
    return this.codec.encode(data);
  }

  decode(data: Uint8Array): DecodedAccount<IDL, N> {
    return this.codec.decode(data);
  }
}
