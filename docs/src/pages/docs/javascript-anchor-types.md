---
title: Javascript Anchor Types Reference
description: Anchor - Javascript Anchor Types Reference
---

This reference shows you how Anchor maps Rust types to JavaScript/TypeScript types in the client.

---

{% table %}
* Rust Type
* JavaScript Type
* Example
* Note
---
* `bool`
* `boolean`
* ```javascript
  await program
    .methods
    .init(true)
    .rpc();
  ```
---
* `u64/u128/i64/i128`
* `anchor.BN`
* ```javascript
  await program
    .methods
    .init(new anchor.BN(99))
    .rpc();
    ```
* [https://github.com/indutny/bn.js](https://github.com/indutny/bn.js )
---
* `u8/u16/u32/i8/i16/i32`
* `number`
* ```javascript
  await program
    .methods
    .init(99)
    .rpc();
    ```
---
* `f32/f64`
* `number`
* ```javascript
  await program
    .methods
    .init(1.0)
    .rpc();
    ```
---
* `Enum`
* `object`
* ```rust
  enum MyEnum {
      One,
      Two { val: u32 },
      Three(u8, i16),
  };
  ```
  ```javascript
  // Unit variant
  await program
    .methods
    .init({ one: {} })
    .rpc();

  // Named variant
  await program
    .methods
    .init({ two: { val: 99 } })
    .rpc();

  // Unnamed (tuple) variant
  await program
    .methods
    .init({ three: [12, -34] })
    .rpc();
  ```
---
* `Struct`
* `{ val: {} }`
* ```rust
  struct MyStruct {
    val: u16,
  }
  ```
  ```javascript
  await program
    .methods
    .init({ val: 99 })
    .rpc();
  ```
---
* `[T; N]`
* `Array<T>`
* ```javascript
  await program
    .methods
    .init([1, 2, 3])
    .rpc();
  ```
---
* `String`
* `string`
* ```javascript
  await program
    .methods
    .init("hello")
    .rpc();
  ```
---
* `Vec<T>`
* `Array<T>`
* ```javascript
  await program
    .methods
    .init([1, 2, 3])
    .rpc();
  ```
{% /table %}
