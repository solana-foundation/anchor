import { IdlError } from '../../error';
import type { Idl, IdlDiscriminator } from '../../types';
import { type IdlCodec, typeDefLayout } from './idl';

export function getTypeDefLayoutByName(
  idl: Idl,
  name: string,
  kind: 'account' | 'event',
): { discriminator?: IdlDiscriminator; layout: IdlCodec } {
  const { types } = idl;

  if (!types) {
    throw new IdlError(`${capitalize(kind)}s require \`idl.types\``);
  }

  const typeDef = types.find((item) => item.name === name);

  if (!typeDef) {
    throw new IdlError(`${capitalize(kind)} not found: ${name}`);
  }

  return {
    layout: typeDefLayout({ typeDef, types }),
  };
}

export function assertDiscriminator(
  actualData: Uint8Array,
  expectedDiscriminator: IdlDiscriminator,
  errorMessage: string,
) {
  const expected = Buffer.from(expectedDiscriminator);
  const actual = actualData.subarray(0, expected.length);

  if (expected.compare(actual) !== 0) {
    throw new IdlError(errorMessage);
  }
}

export function stripDiscriminator(data: Uint8Array, discriminator: IdlDiscriminator): Uint8Array {
  return data.subarray(discriminator.length);
}

function capitalize(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}
