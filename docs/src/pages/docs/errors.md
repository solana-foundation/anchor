---
title: Errors
description: Anchor - Errors
---

> [`AnchorError` Rust Reference](https://docs.rs/anchor-lang/latest/anchor_lang/error/struct.AnchorError.html)

> [`AnchorError` Typescript Reference](https://coral-xyz.github.io/anchor/ts/classes/AnchorError.html)

There are two types of errors in anchor programs. AnchorErrors and non-anchor errors.
AnchorErrors can be divided into Anchor Internal Errors that the framework returns from inside its own code or
custom errors which the user (you!) can return.

- AnchorErrors
  - Anchor Internal Errors
  - Custom Errors
- Non-anchor errors.

[AnchorErrors](https://docs.rs/anchor-lang/latest/anchor_lang/error/struct.AnchorError.html) provide a range of information like the error name and number or the location in the code where the anchor was thrown, or the account that violated a constraint (e.g. a `mut` constraint). Once thrown inside the program, [you can access the error information](https://coral-xyz.github.io/anchor/ts/classes/AnchorError.html) in the anchor clients like the typescript client. The typescript client also enriches the error with additional information about which program the error was thrown in and the CPI calls (which are explained [here](./cross-program-invocations) in the book) that led to the program from which the error was thrown from. [The milestone chapter](./milestone_project_tic-tac-toe.md) explores how all of this works together in practice. For now, let's look at how different errors can be returned from inside a program.

## Anchor Internal Errors

> [Anchor Internal Error Code Reference](https://docs.rs/anchor-lang/latest/anchor_lang/error/enum.ErrorCode.html)

Anchor has many different internal error codes. These are not meant to be used by users, but it's useful to study the reference to learn about the mappings between codes and their causes. They are, for example, thrown when a constraint has been violated, e.g. when an account is marked with `mut` but its `is_writable` property is `false`.

## Custom Errors

You can add errors that are unique to your program by using the `error_code` attribute.

Simply add it to an enum with a name of your choice. You can then use the variants of the enum as errors in your program. Additionally, you can add a message attribute to the individual variants. Clients will then display this error message if the error occurs. Custom Error code numbers start at the [custom error offset](https://docs.rs/anchor-lang/latest/anchor_lang/error/constant.ERROR_CODE_OFFSET.html).

To actually throw an error use the [`err!`](https://docs.rs/anchor-lang/latest/anchor_lang/macro.err.html) or the [`error!`](https://docs.rs/anchor-lang/latest/anchor_lang/prelude/macro.error.html) macro. These add file and line information to the error that is then logged by anchor.

```rust
#[program]
mod hello_anchor {
    use super::*;
    pub fn set_data(ctx: Context<SetData>, data: MyAccount) -> Result<()> {
        if data.data >= 100 {
            return err!(MyError::DataTooLarge);
        }
        ctx.accounts.my_account.set_inner(data);
        Ok(())
    }
}


#[error_code]
pub enum MyError {
    #[msg("MyAccount may only hold data below 100")]
    DataTooLarge
}
```

### require!

You can use the [`require`](https://docs.rs/anchor-lang/latest/anchor_lang/macro.require.html) macro to simplify writing errors. The code above can be simplified to this (Note that the `>=` flips to `<`):

```rust
#[program]
mod hello_anchor {
    use super::*;
    pub fn set_data(ctx: Context<SetData>, data: MyAccount) -> Result<()> {
        require!(data.data < 100, MyError::DataTooLarge);
        ctx.accounts.my_account.set_inner(data);
        Ok(())
    }
}


#[error_code]
pub enum MyError {
    #[msg("MyAccount may only hold data below 100")]
    DataTooLarge
}
```

There are a couple of `require` macros to choose from ([search for require in the docs](https://docs.rs/anchor-lang/latest/anchor_lang/?search=require)). When comparing public keys, it's important to use the `keys` variants of the require statements like `require_keys_eq` instead of `require_eq` because comparing public keys with `require_eq` is very expensive.

> (Ultimately, all programs return the same Error: The [`ProgramError`](https://docs.rs/solana-program/latest/solana_program/program_error/enum.ProgramError.html).

This Error has a field for a custom error number. This is where Anchor puts its internal and custom error codes. However, this is just a single number and a single number is only so useful. So in addition, in the case of AnchorErrors, Anchor logs the returned AnchorError and the Anchor clients parse these logs to provide as much information as possible. This is not always possible. For example, there is currently no easy way to get the logs of a `processed` transaction with preflight checks turned off. In addition, non-anchor or old anchor programs might not log AnchorErrors. In these cases, Anchor will fall back to checking whether the returned error number by the transaction matches an error number defined in the `IDL` or an Anchor internal error code. If so, Anchor will at least enrich the error with the error message. Also, if there are logs available, Anchor will always try to parse the program error stack and return that so you know which program the error was returned from.
