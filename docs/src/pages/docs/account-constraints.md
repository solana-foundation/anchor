---
title: Account Constraints
description: Anchor Account Constraint Examples
---

Minimal reference examples for Anchor account [constraints](https://docs.rs/anchor-lang/latest/anchor_lang/derive.Accounts.html).

## Instruction Attribute

{% table %}

- Attribute
- Example
- Description

---

- ```rust
  #[derive(Accounts)]
  #[instruction(...)]
  pub struct Initialize<'info> {
    ...
  }
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/instruction)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/instruction)
- You can access the instruction’s arguments with the #[instruction(..)] attribute.
  You have to list them in the same order as in the instruction but you can omit all arguments after the last one you need.

{% /table %}

## Normal Constraints

{% table %}

- Attribute
- Example
- Description

---

- ```rust
  #[account(signer)]
  #[account(signer @ <custom_error>)]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/signer)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/signer)
- Checks the given account signed the transaction. Custom errors are supported via @. Consider using the Signer type if you would only have this constraint on the account.

---

- ```rust
  #[account(mut)]
  #[account(mut @ <custom_error>)]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/mut)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/mut)
- Checks the given account is mutable.
  Makes anchor persist any state changes.
  Custom errors are supported via @.

---

- ```rust
  #[account(
    init,
    payer = <target_account>,
    space = <num_bytes>
  )]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/init)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/init)
- Creates the account via a CPI to the system program and initializes it (sets its account discriminator).

---

- ```rust
  #[account(
    init_if_needed,
    payer = <target_account>
  )]

  #[account(
    init_if_needed,
    payer = <target_account>,
    space = <num_bytes>
  )]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/init_if_needed)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/init_if_needed)
- Exact same functionality as the init constraint but only runs if the account does not exist yet.

  This feature should be used with care and is therefore behind a feature flag. You can enable it by importing anchor-lang with the init-if-needed cargo feature.
  When using init_if_needed, you need to make sure you properly protect yourself against re-initialization attacks.

---

- ```rust
  #[account(
    seeds = <seeds>,
    bump
  )]

  #[account(
    seeds = <seeds>,
    bump,
    seeds::program = <expr>
  )]

  #[account(
    seeds = <seeds>,
    bump = <expr>
  )]

  #[account(
    seeds = <seeds>,
    bump = <expr>,
    seeds::program = <expr>
  )]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/seed-bump)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/seed-bump)
- Checks that given account is a PDA derived from the currently executing program, the seeds, and if provided, the bump.
  If not provided, anchor uses the canonical bump.
  Add seeds::program = <expr> to derive the PDA from a different program than the currently executing one.

---

- ```rust
  #[account(
    has_one = <target_account>
  )]

  #[account(
    has_one = <target_account> @ <custom_error>
  )]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/has_one)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/has_one)
- Checks the target_account field on the account matches the key of the target_account field in the Accounts struct.
  Custom errors are supported via @.

---

- ```rust
  #[account(address = <expr>)]
  #[account(address = <expr> @ <custom_error>)]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/address)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/address)
- Checks the account key matches the pubkey.
  Custom errors are supported via @.

---

- ```rust
  #[account(owner = <expr>)]
  #[account(owner = <expr> @ <custom_error>)]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/owner)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/owner)
- Checks the account owner matches expr.
  Custom errors are supported via @.

---

- ```rust
  #[account(executable)]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/executable)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/executable)
- Checks the account is executable (i.e. the account is a program).
  You may want to use the Program type instead.

---

- ```rust
  #[account(rent_exempt = skip)]
  #[account(rent_exempt = enforce)]
  ```

- Github
  Solpg
- Enforces rent exemption with = enforce.
  Skips rent exemption check that would normally be done through other constraints with = skip, e.g. when used with the zero constraint

---

- ```rust
  #[account(zero)]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/zero)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/zero)
- Checks the account discriminator is zero.

  Use this constraint if you want to create an account in a previous instruction and then initialize it in your instruction instead of using init. This is necessary for accounts that are larger than 10 Kibibyte because those accounts cannot be created via a CPI (which is what init would do).

---

- ```rust
  #[account(close = <target_account>)]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/close)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/close)
- Closes the account by:

  - Sending the lamports to the specified account
  - Assigning the owner to the System Program
  - Resetting the data of the account

  Requires mut to exist on the account.

---

- ```rust
  #[account(constraint = <expr>)]
  #[account(
    constraint = <expr> @ <custom_error>
  )]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/constraint)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/constraint)
- Constraint that checks whether the given expression evaluates to true.
  Use this when no other constraint fits your use case.

---

- ```rust
  #[account(
    realloc = <space>,
    realloc::payer = <target>,
    realloc::zero = <bool>
  )]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/realloc)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/realloc)
- Used to realloc program account space at the beginning of an instruction.

{% /table %}

## SPL Constraints

{% table %}

- Attribute
- Example
- Description

---

- ```rust
  #[account(
    token::mint = <target_account>,
    token::authority = <target_account>
  )]

  #[account(
    token::mint = <target_account>,
    token::authority = <target_account>,
    token::token_program = <target_account>
  )]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/token)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/token)
- Can be used as a check or with init to create a token account with the given mint address and authority.
  When used as a check, it's possible to only specify a subset of the constraints.

---

- ```rust
  #[account(
    mint::authority = <target_account>,
    mint::decimals = <expr>
  )]

  #[account(
    mint::authority = <target_account>,
    mint::decimals = <expr>,
    mint::freeze_authority = <target_account>
  )]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/mint)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/mint)
- Can be used as a check or with init to create a mint account with the given mint decimals and mint authority.
  The freeze authority is optional when used with init.
  When used as a check, it's possible to only specify a subset of the constraints.

---

- ```rust
  #[account(
    associated_token::mint = <target_account>,
    associated_token::authority = <target_account>
  )]

  #[account(
    associated_token::mint = <target_account>,
    associated_token::authority = <target_account>,
    associated_token::token_program = <target_account>
  )]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/associated_token)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/associated_token)
- Can be used as a standalone as a check or with init to create an associated token account with the given mint address and authority.

---

- ```rust
  #[account(*::token_program = <target_account>)]
  ```

- [Github](https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/token_program)
  [Solpg](https://beta.solpg.io/https://github.com/solana-developers/anchor-examples/tree/main/account-constraints/token_program)
- The token_program can optionally be overridden.

{% /table %}
