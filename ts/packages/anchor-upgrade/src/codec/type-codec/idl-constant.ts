import { IdlError } from '../../error';
import type { IdlField, IdlType, IdlTypeDef } from '../../types';

export function decodeIdlConstValue(type: IdlType, rawValue: string, types: IdlTypeDef[]): unknown {
  rawValue = rawValue.trim();

  switch (type) {
    case 'string':
    case 'pubkey':
      return rawValue;

    case 'bool':
      return parseBoolean(rawValue);

    case 'u8':
    case 'i8':
    case 'u16':
    case 'i16':
    case 'u32':
    case 'i32':
    case 'f32':
    case 'f64':
      return Number(rawValue);

    case 'u64':
    case 'i64':
    case 'u128':
    case 'i128':
    case 'u256':
    case 'i256':
      return rawValue.at(0) === '-' ? Number(rawValue) : BigInt(rawValue);

    case 'bytes':
      return Uint8Array.from(parseJson<number[]>(rawValue));

    default:
      if ('option' in type) {
        return decodeOptionalValue(type.option, rawValue, types);
      }

      if ('coption' in type) {
        return decodeOptionalValue(type.coption, rawValue, types);
      }

      if ('vec' in type) {
        return decodeArrayValues(type.vec, rawValue, types);
      }

      if ('array' in type) {
        const [elementType] = type.array;
        return decodeArrayValues(elementType, rawValue, types);
      }

      if ('defined' in type) {
        return decodeTypeDefValue(requireTypeDef(types, type.defined.name), rawValue, types);
      }

      if ('generic' in type) {
        throw new IdlError(`Generic constants are not supported: ${type.generic}`);
      }

      throw new IdlError(`Constant decoding is not implemented for type: ${JSON.stringify(type)}`);
  }
}

function decodeOptionalValue(innerType: IdlType, rawValue: string, types: IdlTypeDef[]): unknown {
  if (rawValue === 'null' || rawValue === 'None') {
    return null;
  }

  const someMatch = rawValue.match(/^Some\((.*)\)$/s);

  return decodeIdlConstValue(innerType, someMatch ? someMatch[1]! : rawValue, types);
}

function decodeArrayValues(elementType: IdlType, rawValue: string, types: IdlTypeDef[]): unknown[] {
  return expectArray(parseJson(rawValue), rawValue).map((value) =>
    decodeFromUnknown(elementType, value, types),
  );
}

function decodeTypeDefValue(typeDef: IdlTypeDef, rawValue: string, types: IdlTypeDef[]): unknown {
  switch (typeDef.type.kind) {
    case 'type':
      return decodeIdlConstValue(typeDef.type.alias, rawValue, types);

    case 'struct': {
      const fields = typeDef.type.fields;

      if (!fields) {
        return {};
      }

      const parsed = parseJson(rawValue);

      if (isNamedFields(fields)) {
        const record = expectRecord(parsed, rawValue);

        return Object.fromEntries(
          fields.map((field) => [
            field.name,
            decodeFromUnknown(field.type, record[field.name], types),
          ]),
        );
      }

      const tupleValues = expectArray(parsed, rawValue);

      return fields.map((fieldType, index) =>
        decodeFromUnknown(fieldType, tupleValues[index], types),
      );
    }

    case 'enum': {
      const parsed = parseJson(rawValue);

      if (typeof parsed === 'string') {
        const variant = requireEnumVariant(typeDef, parsed);

        if (variant.fields) {
          throw new IdlError(`Enum variant ${parsed} in ${typeDef.name} requires payload`);
        }

        return { [parsed]: {} };
      }

      const record = expectRecord(parsed, rawValue);
      const entries = Object.entries(record);

      if (entries.length !== 1) {
        throw new IdlError(`Enum ${typeDef.name} must contain exactly one variant`);
      }

      const [variantName, variantPayload] = entries[0]!;
      const variant = requireEnumVariant(typeDef, variantName);

      if (!variant.fields) {
        return { [variantName]: {} };
      }

      if (isNamedFields(variant.fields)) {
        const payloadRecord = expectRecord(variantPayload, JSON.stringify(variantPayload));

        return {
          [variantName]: Object.fromEntries(
            variant.fields.map((field) => [
              field.name,
              decodeFromUnknown(field.type, payloadRecord[field.name], types),
            ]),
          ),
        };
      }

      const tuplePayload = expectArray(variantPayload, JSON.stringify(variantPayload));

      return {
        [variantName]: variant.fields.map((fieldType, index) =>
          decodeFromUnknown(fieldType, tuplePayload[index], types),
        ),
      };
    }
  }
}

function decodeFromUnknown(type: IdlType, value: unknown, types: IdlTypeDef[]): unknown {
  return decodeIdlConstValue(type, JSON.stringify(value), types);
}

function requireTypeDef(types: IdlTypeDef[], name: string): IdlTypeDef {
  const typeDef = types.find((item) => item.name === name);

  if (!typeDef) {
    throw new IdlError(`Type not found: ${name}`);
  }

  return typeDef;
}

function requireEnumVariant(typeDef: IdlTypeDef, variantName: string) {
  if (typeDef.type.kind !== 'enum') {
    throw new IdlError(`Type ${typeDef.name} is not an enum`);
  }

  const variant = typeDef.type.variants.find((item) => item.name === variantName);

  if (!variant) {
    throw new IdlError(`Variant not found in ${typeDef.name}: ${variantName}`);
  }

  return variant;
}

function parseJson<T>(rawValue: string): T {
  try {
    return JSON.parse(rawValue) as T;
  } catch {
    throw new IdlError(`Invalid JSON constant value: ${rawValue}`);
  }
}

function parseBoolean(rawValue: string): boolean {
  if (rawValue === 'true') return true;
  if (rawValue === 'false') return false;

  throw new IdlError(`Invalid bool constant: ${rawValue}`);
}

function expectArray(value: unknown, rawValue: string): unknown[] {
  if (!Array.isArray(value)) {
    throw new IdlError(`Expected array constant: ${rawValue}`);
  }

  return value;
}

function expectRecord(value: unknown, rawValue: string): Record<string, unknown> {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new IdlError(`Expected object constant: ${rawValue}`);
  }

  return value as Record<string, unknown>;
}

function isNamedFields(fields: readonly unknown[]): fields is readonly IdlField[] {
  return fields.every(
    (field) => typeof field === 'object' && field !== null && 'name' in field && 'type' in field,
  );
}
