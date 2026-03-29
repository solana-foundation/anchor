import type { Idl, IdlDiscriminator, NullableIdlAccount } from '../types';
import type { IdlCodec } from './type-codec';
import {
  assertDiscriminator,
  getTypeDefLayoutByName,
  stripDiscriminator,
} from './type-codec/helpers';

export class AccountCodec<T = unknown> {
  private readonly discriminator: IdlDiscriminator;
  private readonly layout: IdlCodec;

  constructor(idl: Idl, accountDef: NullableIdlAccount<Idl>) {
    this.discriminator = accountDef.discriminator;
    this.layout = getTypeDefLayoutByName(idl, accountDef.name, 'account').layout;
  }

  encode(accountData: T): Uint8Array {
    const encodedRaw = this.layout.encode(accountData);
    const encoded = new Uint8Array(
      encodedRaw.buffer,
      encodedRaw.byteOffset,
      encodedRaw.byteLength
    );

    return Uint8Array.from([...this.discriminator, ...encoded])
  }

  decode(encodedData: Uint8Array): T {
    assertDiscriminator(encodedData, this.discriminator, 'Invalid account discriminator');

    return this.layout.decode(stripDiscriminator(encodedData, this.discriminator)) as T;
  }
}
