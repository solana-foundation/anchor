import {
  addCodecSizePrefix,
  type Codec,
  getAddressCodec,
  getArrayCodec,
  getBooleanCodec,
  getDiscriminatedUnionCodec,
  getF32Codec,
  getF64Codec,
  getI8Codec,
  getI16Codec,
  getI32Codec,
  getI64Codec,
  getI128Codec,
  getOptionCodec,
  getStructCodec,
  getU8Codec,
  getU16Codec,
  getU32Codec,
  getU64Codec,
  getU128Codec,
  getUnitCodec,
  getUtf8Codec,
  type Option,
  transformCodec,
} from '@solana/kit';

import { IdlError } from '../../error';
import type {
  IdlArrayLen,
  IdlField,
  IdlGenericArg,
  IdlGenericArgConst,
  IdlType,
  IdlTypeDef,
} from '../../types';
import { handleDefinedFields } from '../../utils';

export type IdlCodec = Codec<any, any>;
export type IdlNamedCodec = [string, IdlCodec];

type PartialField = { name?: string } & Pick<IdlField, 'type'>;

function withFieldName(codec: IdlCodec, fieldName?: string): IdlNamedCodec | IdlCodec {
  return fieldName ? [fieldName, codec] : codec;
}

function getStringCodec(): Codec<string> {
  return addCodecSizePrefix(getUtf8Codec(), getU32Codec());
}

function getU256Codec(): Codec<bigint> {
  throw new IdlError('u256 is not provided by @solana/kit; implement a custom 32-byte LE codec');
}

function getI256Codec(): Codec<bigint> {
  throw new IdlError('i256 is not provided by @solana/kit; implement a custom 32-byte LE codec');
}

function resolveArrayLen(len: IdlArrayLen, genericArgs?: IdlGenericArg[] | null): number {
  if (typeof len === 'number') return len;

  if (genericArgs) {
    const genericLen = genericArgs.find((g) => g.kind === 'const');
    if (genericLen?.kind === 'const') {
      len = +genericLen.value;
    }
  }

  if (typeof len !== 'number') {
    throw new IdlError('Generic array length did not resolve');
  }

  return len;
}

function getNullableOptionCodec(innerCodec: IdlCodec): IdlCodec {
  return transformCodec(
    getOptionCodec(innerCodec),
    (value: unknown | null) =>
      value === null ? { __option: 'None' } : { __option: 'Some', value },
    (value: Option<unknown>) => (value.__option === 'Some' ? value.value : null),
  );
}

function resolveGenericArgs({
  type,
  typeDef,
  genericArgs,
  isDefined,
}: {
  type: IdlType;
  typeDef: IdlTypeDef;
  genericArgs: IdlGenericArg[];
  isDefined?: boolean | undefined;
}): IdlGenericArg[] | null {
  if (typeof type !== 'object') return null;

  const defGenerics = typeDef.generics ?? [];

  for (const [index, defGeneric] of defGenerics.entries()) {
    if ('generic' in type && defGeneric.name === type.generic) {
      return [genericArgs[index]!];
    }

    if ('option' in type) {
      const args = resolveGenericArgs({
        type: type.option,
        typeDef,
        genericArgs,
        isDefined,
      });

      if (!args || !isDefined) return args;

      if (args[0]?.kind === 'type') {
        return [{ kind: 'type', type: { option: args[0].type } }];
      }
    }

    if ('vec' in type) {
      const args = resolveGenericArgs({
        type: type.vec,
        typeDef,
        genericArgs,
        isDefined,
      });

      if (!args || !isDefined) return args;

      if (args[0]?.kind === 'type') {
        return [{ kind: 'type', type: { vec: args[0].type } }];
      }
    }

    if ('array' in type) {
      const [elTy, len] = type.array;
      const isGenericLen = typeof len === 'object';

      const args =
        resolveGenericArgs({
          type: elTy,
          typeDef,
          genericArgs,
          isDefined,
        }) ?? [];

      if (isGenericLen) {
        const matchingGeneric = defGenerics.findIndex((g) => g.name === len.generic);
        if (matchingGeneric !== -1) {
          args.push(genericArgs[matchingGeneric]!);
        }
      }

      if (args.length > 0) {
        if (!isDefined) return args;

        if (args[0]?.kind === 'type' && args[1]?.kind === 'const') {
          return [
            {
              kind: 'type',
              type: { array: [args[0].type, +args[1].value] },
            },
          ];
        }
      }

      if (isGenericLen && defGeneric.name === len.generic) {
        const arg = genericArgs[index]!;
        if (!isDefined) return [arg];

        return [
          {
            kind: 'type',
            type: { array: [elTy, +(arg as IdlGenericArgConst).value] },
          },
        ];
      }

      return null;
    }

    if ('defined' in type) {
      if (!type.defined.generics) return null;

      return type.defined.generics
        .flatMap((g) => {
          switch (g.kind) {
            case 'type':
              return (
                resolveGenericArgs({
                  type: g.type,
                  typeDef,
                  genericArgs,
                  isDefined: true,
                }) ?? []
              );
            case 'const':
              return [g];

            default:
              return [];
          }
        })
        .filter(Boolean) as IdlGenericArg[];
    }
  }

  return null;
}

export function fieldCodec(
  type: IdlType,
  types: IdlTypeDef[] = [],
  genericArgs?: IdlGenericArg[] | null,
): IdlCodec {
  switch (type) {
    case 'bool':
      return getBooleanCodec();
    case 'u8':
      return getU8Codec();
    case 'i8':
      return getI8Codec();
    case 'u16':
      return getU16Codec();
    case 'i16':
      return getI16Codec();
    case 'u32':
      return getU32Codec();
    case 'i32':
      return getI32Codec();
    case 'f32':
      return getF32Codec();
    case 'u64':
      return getU64Codec();
    case 'i64':
      return getI64Codec();
    case 'f64':
      return getF64Codec();
    case 'u128':
      return getU128Codec();
    case 'i128':
      return getI128Codec();
    case 'u256':
      return getU256Codec();
    case 'i256':
      return getI256Codec();
    case 'bytes':
      return getArrayCodec(getU8Codec(), { size: getU32Codec() });
    case 'string':
      return getStringCodec();
    case 'pubkey':
      return getAddressCodec();
    default: {
      // if ('сoption' in type) {
      //   return getNullableOptionCodec(fieldCodec(type.сoption, types, genericArgs));
      // }

      if ('option' in type) {
        return getNullableOptionCodec(fieldCodec(type.option, types, genericArgs));
      }

      if ('vec' in type) {
        return getArrayCodec(fieldCodec(type.vec, types, genericArgs), {
          size: getU32Codec(),
        });
      }

      if ('array' in type) {
        const [innerType, len] = type.array;
        return getArrayCodec(fieldCodec(innerType, types, genericArgs), {
          size: resolveArrayLen(len, genericArgs),
        });
      }

      if ('defined' in type) {
        const typeDef = types.find((t) => t.name === type.defined.name);
        if (!typeDef) {
          throw new IdlError(`Type not found: ${type.defined.name}`);
        }

        return typeDefLayout({
          typeDef,
          types,
          genericArgs: genericArgs ?? type.defined.generics ?? null,
        });
      }

      if ('generic' in type) {
        const genericArg = genericArgs?.at(0);
        if (genericArg?.kind !== 'type') {
          throw new IdlError(`Invalid generic field: ${type.generic}`);
        }
        return fieldCodec(genericArg.type, types);
      }

      throw new IdlError(`Not yet implemented: ${JSON.stringify(type)}`);
    }
  }
}

export function fieldLayout(
  field: PartialField,
  types: IdlTypeDef[] = [],
  genericArgs?: IdlGenericArg[] | null,
): IdlNamedCodec | IdlCodec {
  return withFieldName(fieldCodec(field.type, types, genericArgs), field.name);
}

export function typeDefLayout({
  typeDef,
  types,
  genericArgs,
}: {
  typeDef: IdlTypeDef;
  types: IdlTypeDef[];
  genericArgs?: IdlGenericArg[] | null;
}): IdlCodec {
  switch (typeDef.type.kind) {
    case 'struct': {
      const fieldLayouts = handleDefinedFields(
        typeDef.type.fields,
        () => [],
        (fields) =>
          fields.map((f) => {
            const genArgs = genericArgs
              ? resolveGenericArgs({
                  type: f.type,
                  typeDef,
                  genericArgs,
                })
              : genericArgs;
            return fieldLayout(f, types, genArgs) as IdlNamedCodec;
          }),
        (fields) =>
          fields.map((f, i) => {
            const genArgs = genericArgs
              ? resolveGenericArgs({
                  type: f,
                  typeDef,
                  genericArgs,
                })
              : genericArgs;
            return fieldLayout({ name: i.toString(), type: f }, types, genArgs) as IdlNamedCodec;
          }),
      );

      return getStructCodec(fieldLayouts);
    }

    case 'enum': {
      const variants = typeDef.type.variants.map((variant) => {
        const variantCodec = handleDefinedFields(
          variant.fields,
          () => getUnitCodec(),
          (fields) =>
            getStructCodec(
              fields.map((f) => {
                const genArgs = genericArgs
                  ? resolveGenericArgs({
                      type: f.type,
                      typeDef,
                      genericArgs,
                    })
                  : genericArgs;

                return fieldLayout(f, types, genArgs) as IdlNamedCodec;
              }),
            ),
          (fields) =>
            getStructCodec(
              fields.map((f, i) => {
                const genArgs = genericArgs
                  ? resolveGenericArgs({
                      type: f,
                      typeDef,
                      genericArgs,
                    })
                  : genericArgs;

                return fieldLayout(
                  { name: i.toString(), type: f },
                  types,
                  genArgs,
                ) as IdlNamedCodec;
              }),
            ),
        );

        return [variant.name, variantCodec] as const;
      });

      return getDiscriminatedUnionCodec(variants as never, {
        discriminator: '__kind',
        size: getU8Codec(),
      }) as IdlCodec;
    }

    case 'type':
      return fieldCodec(typeDef.type.alias, types, genericArgs);
  }
}
