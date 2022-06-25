---
title: The Accounts Struct
description: Anchor - The Accounts Struct
---

The Accounts struct is where you define which accounts your instruction expects and which constraints these accounts should adhere to. You do this via two constructs: Types and constraints.

---

## Types

> [Account Types Reference](https://docs.rs/anchor-lang/latest/anchor_lang/accounts/index.html)

Each type has a specific use case in mind. Detailed explanations for the types can be found in the [reference](https://docs.rs/anchor-lang/latest/anchor_lang/accounts/index.html). We will briefly explain the most important type here, the `Account` type.

### The Account Type

> [Account Reference](https://docs.rs/anchor-lang/latest/anchor_lang/accounts/account/struct.Account.html)

The `Account` type is used when an instruction is interested in the deserialized data of the account. Consider the following example where we set some data in an account:

```rust
use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
mod hello_anchor {
    use super::*;
    pub fn set_data(ctx: Context<SetData>, data: u64) -> Result<()> {
        ctx.accounts.my_account.data = data;
        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct MyAccount {
    data: u64
}

#[derive(Accounts)]
pub struct SetData<'info> {
    #[account(mut)]
    pub my_account: Account<'info, MyAccount>
}
```

`Account` is generic over `T`. This `T` is a type you can create yourself to store data. In this example, we have created a struct `MyAccount` with a single `data` field to store a `u64`. Account requires `T` to implement certain functions (e.g. functions that (de)serialize `T`). Most of the time, you can use the `#[account]` attribute to add these functions to your data, as is done in the example.

Most importantly, the `#[account]` attribute sets the owner of that data to the `ID` (the one we created earlier with `declare_id`) of the crate `#[account]` is used in. The Account type can then check for you that the `AccountInfo` passed into your instruction has its `owner` field set to the correct program. In this example, `MyAccount` is declared in our own crate so `Account` will verify that the owner of `my_account` equals the address we declared with `declare_id`.

#### Using `Account<'a, T>` with non-anchor program accounts

There may be cases where you want your program to interact with a non-Anchor program. You can still get all the benefits of `Account` but you have to write a custom wrapper type instead of using `#[account]`. For instance, Anchor provides wrapper types for the token program accounts so they can be used with `Account`.

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
mod hello_anchor {
    use super::*;
    pub fn set_data(ctx: Context<SetData>, data: u64) -> Result<()> {
        if ctx.accounts.token_account.amount > 0 {
            ctx.accounts.my_account.data = data;
        }
        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct MyAccount {
    data: u64,
    mint: Pubkey
}

#[derive(Accounts)]
pub struct SetData<'info> {
    #[account(mut)]
    pub my_account: Account<'info, MyAccount>,
    #[account(
        constraint = my_account.mint == token_account.mint,
        has_one = owner
    )]
    pub token_account: Account<'info, TokenAccount>,
    pub owner: Signer<'info>
}
```

To run this example, add `anchor-spl = "<version>"` to the dependencies section in your `Cargo.toml`, located in the `programs/<your-project-name>/` directory. `<version>` should be equal to the `anchor-lang` version you're using.

In this example, we set the `data` field of an account if the caller has admin rights. We decide whether the caller is an admin by checking whether they own admin tokens for the account they want to change. We do most of this via constraints which we will look at in the next section.
The important thing to take away is that we use the `TokenAccount` type (that wraps around the token program's `Account` struct and adds the required functions) to make anchor ensure that the incoming account is owned by the token program and to make anchor deserialize it. This means we can use the `TokenAccount` properties inside our constraints (e.g. `token_account.mint`) as well as in the instruction function.

Check out the [reference for the Account type](https://docs.rs/anchor-lang/latest/anchor_lang/accounts/account/struct.Account.html) to learn how to implement your own wrapper types for non-anchor programs.

## Constraints

> [Constraints reference](https://docs.rs/anchor-lang/latest/anchor_lang/derive.Accounts.html)

Account types can do a lot of work for you but they're not dynamic enough to handle all the security checks a secure program requires.

Add constraints to an account with the following format:

```rust
#[account(<constraints>)]
pub account: AccountType
```

Some constraints support custom Errors (we will explore errors [later](./errors.md)):

```rust
#[account(...,<constraint> @ MyError::MyErrorVariant, ...)]
pub account: AccountType
```

For example, in the examples above, we used the `mut` constraint to indicate that `my_account` should be mutable. We used `has_one` to check that `token_account.owner == owner.key()`. And finally we used `constraint` to check an arbitrary expression; in this case, whether the incoming `TokenAccount` belongs to the admin mint.

```rust
#[derive(Accounts)]
pub struct SetData<'info> {
    #[account(mut)]
    pub my_account: Account<'info, MyAccount>,
    #[account(
        constraint = my_account.mint == token_account.mint,
        has_one = owner
    )]
    pub token_account: Account<'info, TokenAccount>,
    pub owner: Signer<'info>
}
```

You can find information about all constraints in the reference. We will cover some of the most important ones in the milestone project at the end of the Essentials section.

## Safety checks

Two of the Anchor account types, [AccountInfo](https://docs.rs/anchor-lang/latest/anchor_lang/accounts/account_info/index.html) and [UncheckedAccount](https://docs.rs/anchor-lang/latest/anchor_lang/accounts/unchecked_account/index.html) do not implement any checks on the account being passed. Anchor implements safety checks that encourage additional documentation describing why additional checks are not necesssary.

Attempting to build a program containing the following excerpt with `anchor build`:

```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    pub potentially_dangerous: UncheckedAccount<'info>
}
```

will result in an error similar to the following:

```shell
Error:
        /anchor/tests/unchecked/programs/unchecked/src/lib.rs:15:8
        Struct field "potentially_dangerous" is unsafe, but is not documented.
        Please add a `/// CHECK:` doc comment explaining why no checks through types are necessary.
        See https://book.anchor-lang.com/anchor_in_depth/the_accounts_struct.html#safety-checks for more information.
```

To fix this, write a doc comment describing the potential security implications, e.g.:

```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub potentially_dangerous: UncheckedAccount<'info>
}
```

Note the doc comment needs to be a [line or block doc comment](https://doc.rust-lang.org/reference/comments.html#doc-comments) (/// or /\*\*) to be interepreted as doc attribute by Rust. Double slash comments (//) are not interpreted as such.

{% callout type="warning" title="Note" %}
The doc comment needs to be a [line or block doc comment](https://doc.rust-lang.org/reference/comments.html#doc-comments) (/// or /\*\*) to be interepreted as doc attribute by Rust. Double slash comments (//) are not interpreted as such.
{% /callout %}

## Other Resources

- [Solana Cookbook](https://solanacookbook.com/core-concepts/accounts.html)
