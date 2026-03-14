import type { AnyFn, Idl, Primitive } from './types';

type DeepReadonly<T> = T extends Primitive | AnyFn
  ? T
  : T extends readonly [unknown, ...unknown[]]
    ? { readonly [K in keyof T]: DeepReadonly<T[K]> }
    : T extends readonly (infer U)[]
      ? readonly DeepReadonly<U>[]
      : { readonly [K in keyof T]: DeepReadonly<T[K]> };

type DeepMutable<T> = T extends Primitive | AnyFn
  ? T
  : T extends readonly [unknown, ...unknown[]]
    ? { -readonly [K in keyof T]: DeepMutable<T[K]> }
    : T extends readonly (infer U)[]
      ? DeepMutable<U>[]
      : { -readonly [K in keyof T]: DeepMutable<T[K]> };

export function defineIdl<const T extends DeepReadonly<Idl>>(idl: T): DeepMutable<T> {
  return idl as DeepMutable<T>;
}
