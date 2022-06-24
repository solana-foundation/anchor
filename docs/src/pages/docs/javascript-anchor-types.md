---
title: Javascript Anchor Types Reference
description: Anchor - Javascript Anchor Types Reference
---

This reference shows you how anchor maps rust types to javascript/typescript types in the client.

---

{% table %}
* Rust Type
* Javascript Type
* Example
* Note
---
* `bool`
* `bool`
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
* [https://github.com/indutny/bn.js/](https://github.com/indutny/bn.js/ )
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
* `{ variantName: {} }`
*   ```rust
      enum MyEnum { One, Two };
    ```
    ```javascript
    await program
    .methods
    .init({ one: {} })
    .rpc();
    ```
    ```rust
    enum MyEnum { One: { val: u64 }, Two };
    ```
    ```javascript
    await program
    .methods
    .init({ one: { val: 99 } })
    .rpc();
    ```
---
* `Struct`
* `{ val: {} }`
* ```rust
    struct MyStruct { val: u64 };
    ```
  ```javascript
    await program
    .methods
    .init({ val: 99 })
    .rpc();
    ```
---
* `[T; N]`
* `[ T ]`
* ```javascript
    await program
    .methods
    .init([1,2,3])
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
* `[ T ]`
* ```javascript
    await program
    .methods
    .init([1,2,3])
    .rpc();
    ```
{% /table %}
