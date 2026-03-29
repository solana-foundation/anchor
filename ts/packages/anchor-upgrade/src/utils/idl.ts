import camelCase from 'camelcase';

import type {
  CamelizedIdl,
  Idl,
  IdlDefinedFields,
  IdlDefinedFieldsNamed,
  IdlDefinedFieldsTuple,
} from '../types';

const KEYS_TO_CONVERT = ['name', 'path', 'account', 'relations', 'generic'] as const;

const toCamelCase = (s: string): string =>
  s
    .split('.')
    .map((part) => camelCase(part, { locale: false }))
    .join('.');

const convertNamesToCamelCase = (obj: Record<string, unknown>): void => {
  for (const key in obj) {
    const val = obj[key];

    if ((KEYS_TO_CONVERT as readonly string[]).includes(key)) {
      obj[key] = Array.isArray(val)
        ? val.map((item) => toCamelCase(item as string))
        : toCamelCase(val as string);
    } else if (typeof val === 'object' && val !== null) {
      convertNamesToCamelCase(val as Record<string, unknown>);
    }
  }
};

export function convertIdlToCamelCase<const T extends Idl>(idl: T): CamelizedIdl<T> {
  const camelCasedIdl = structuredClone(idl);

  if (typeof camelCasedIdl === 'object' && camelCasedIdl !== null) {
    convertNamesToCamelCase(camelCasedIdl as Record<string, unknown>);
  }

  return camelCasedIdl as CamelizedIdl<T>;
}

function isNamedFields(fields: IdlDefinedFields): fields is IdlDefinedFieldsNamed {
  return Boolean(fields[0] && typeof fields[0] === 'object' && 'name' in fields[0]);
}

export function handleDefinedFields<U, N, T>(
  fields: IdlDefinedFields | undefined,
  unitCb: () => U,
  namedCb: (fields: IdlDefinedFieldsNamed) => N,
  tupleCb: (fields: IdlDefinedFieldsTuple) => T,
): U | N | T {
  // Unit
  if (!fields?.length) return unitCb();

  // Named
  if (isNamedFields(fields)) {
    return namedCb(fields);
  }

  // Tuple
  return tupleCb(fields);
}
