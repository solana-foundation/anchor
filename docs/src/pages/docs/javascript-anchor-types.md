---
title: Javascript Anchor Types Reference
description: Anchor - Javascript Anchor Types Reference
---

This reference shows you how Anchor maps Rust types to JavaScript/TypeScript types in the client.

---

{% table %}
* Type
* Rust
* TypeScript
* Example
---
* ## Boolean
* `bool`
* `boolean`
* ```typescript
  true
  ```
---
* ## Integer
* `u8/u16/u32/i8/i16/i32`
* `number`
* ```typescript
  99
  ```
---
* ## Big integer
* `u64/u128/i64/i128`
* `anchor.BN`
* ```typescript
  new anchor.BN(99)
  ```
---
* ## Float
* `f32/f64`
* `number`
* ```typescript
  1.0
  ```
---
* ## String
* `String`
* `string`
* ```typescript
  "hello"
  ```
---
* ## Array
* `[T; N]`
* `Array<T>`
* ```typescript
  [1, 2, 3]
  ```
---
* ## Vector
* `Vec<T>`
* `Array<T>`
* ```typescript
  [1, 2, 3]
  ```
---
* ## Option
* `Option<T>`
* `T | null | undefined`
* `None`:
  ```typescript
  null
  ```
  `Some(val)`:
  ```typescript
  42
  ```
---
* ## Struct
* `Struct`
* `object`
* ```rust
  struct MyStruct {
    val: u16,
  }
  ```
  ```typescript
  { val: 99 }
  ```
---
* ## Enum
* `Enum`
* `object`
* ```rust
  enum MyEnum {
      One,
      Two { val: u32 },
      Three(u8, i16),
  }
  ```
  Unit variant:
  ```typescript
  { one : {} }
  ```
  Named variant:
  ```typescript
  { two: { val: 99 } }
  ```
  Unnamed (tuple) variant:
  ```typescript
  { three: [12, -34] }
  ```
{% /table %}
