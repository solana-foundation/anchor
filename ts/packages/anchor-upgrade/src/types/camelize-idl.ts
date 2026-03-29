import type { Primitive } from './common';

type TargetKey = 'relations' | 'account' | 'path' | 'name' | 'generic';

type CamelTail<S extends string> = S extends `${infer Head}_${infer Tail}`
  ? `${Capitalize<Lowercase<Head>>}${CamelTail<Tail>}`
  : Capitalize<Lowercase<S>>;

type CamelCase<S extends string> = S extends `_${infer Rest}`
  ? `_${CamelCase<Rest>}`
  : S extends `${infer Head}_${infer Tail}`
    ? `${Lowercase<Head>}${CamelTail<Tail>}`
    : Uncapitalize<S>;

type WalkField<K extends PropertyKey, V> = K extends TargetKey
  ? V extends string
    ? CamelCase<V>
    : RecursiveWalk<V>
  : V extends Primitive
    ? V
    : RecursiveWalk<V>;

type RecursiveWalk<T> = T extends Primitive
  ? T
  : T extends readonly unknown[]
    ? { [I in keyof T]: RecursiveWalk<T[I]> }
    : T extends object
      ? { [K in keyof T]: WalkField<K, T[K]> }
      : T;

export type CamelizedIdl<T> = RecursiveWalk<T>;
