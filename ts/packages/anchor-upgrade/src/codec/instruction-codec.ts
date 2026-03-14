import { getStructCodec } from '@solana/kit';

import type { Idl, IdlDiscriminator, IdlInstruction } from '../types';
import { fieldLayout, type IdlCodec, type IdlNamedCodec } from './type-codec';
import { assertDiscriminator, stripDiscriminator } from './type-codec/helpers';

export class InstructionCodec<T = unknown> {
  private readonly discriminator: IdlDiscriminator;
  private readonly layout: IdlCodec;

  constructor(idl: Idl, instructionDef: IdlInstruction) {
    const fieldCodecs = instructionDef.args.map(
      (arg) => fieldLayout(arg, idl.types) as IdlNamedCodec,
    );

    this.discriminator = instructionDef.discriminator;
    this.layout = getStructCodec(fieldCodecs);
  }

  encode(instructionData: T): Uint8Array {
    const encodedRaw = this.layout.encode(instructionData);
    const encoded = new Uint8Array(
      encodedRaw.buffer,
      encodedRaw.byteOffset,
      encodedRaw.byteLength
    );

    return Uint8Array.from([...this.discriminator, ...encoded])
  }

  decode(encodedData: Uint8Array): T {
    assertDiscriminator(encodedData, this.discriminator, 'Invalid instruction discriminator');

    return this.layout.decode(stripDiscriminator(encodedData, this.discriminator)) as T;
  }
}
