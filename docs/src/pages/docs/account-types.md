---
title: Account Types
description: Anchor Account Type Examples
---

Minimal reference examples for Anchor [account types](https://docs.rs/anchor-lang/latest/anchor_lang/accounts/index.html).

{% table %}

- Type
- Example
- Description

---

- ```rust
  Account<'info, T>
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-types/Account)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-types/Account)
- Account container that checks ownership on deserialization

---

- ```rust
  AccountInfo<'info>
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-types/AccountInfo)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-types/AccountInfo)
- AccountInfo can be used as a type but Unchecked Account should be used instead

---

- ```rust
  AccountLoader<'info, T>
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-types/AccountLoader)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-types/AccountLoader)
- Type facilitating on demand zero copy deserialization

---

- ```rust
  Box<Account<'info, T>>
  Box<InterfaceAccount<'info, T>>
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-types/Box)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-types/Box)
- Box type to save stack space

---

- ```rust
  Interface<'info, T>
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-types/Interface)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-types/Interface)
- Type validating that the account is one of a set of given Programs

---

- ```rust
  InterfaceAccount<'info, T>
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-types/InterfaceAccount)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-types/InterfaceAccount)
- Account container that checks ownership on deserialization

---

- ```rust
  Option<Account<'info, T>>
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-types/Option)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-types/Option)
- Option type for optional accounts

---

- ```rust
  Program<'info, T>
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-types/Program)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-types/Program)
- Type validating that the account is the given Program

---

- ```rust
  Signer<'info>
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-types/Signer)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-types/Signer)
- Type validating that the account signed the transaction

---

- ```rust
  SystemAccount<'info>
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-types/SystemAccount)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-types/SystemAccount)
- Type validating that the account is owned by the system program

---

- ```rust
  Sysvar<'info, T>
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-types/Sysvar)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-types/Sysvar)
- Type validating that the account is a sysvar and deserializing it

---

- ```rust
  UncheckedAccount<'info>
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-types/UncheckedAccount)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-types/UncheckedAccount)
- Explicit wrapper for AccountInfo types to emphasize that no checks are performed

{% /table %}
