import type { Idl, IdlDiscriminator, NullableIdlEvent } from '../types';
import type { IdlCodec } from './type-codec';
import {
  assertDiscriminator,
  getTypeDefLayoutByName,
  stripDiscriminator,
} from './type-codec/helpers';

export class EventCodec<T = unknown> {
  private readonly discriminator: IdlDiscriminator;
  private readonly layout: IdlCodec;

  constructor(idl: Idl, eventDef: NullableIdlEvent<Idl>) {
    this.discriminator = eventDef.discriminator;
    this.layout = getTypeDefLayoutByName(idl, eventDef.name, 'event').layout;
  }

  decode(base64Log: string): T {
    const encodedData = Uint8Array.from(Buffer.from(base64Log, 'base64'));

    assertDiscriminator(encodedData, this.discriminator, 'Invalid event discriminator');

    return this.layout.decode(stripDiscriminator(encodedData, this.discriminator)) as T;
  }
}
