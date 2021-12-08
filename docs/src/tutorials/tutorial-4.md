# Errors

If you've ever programmed on a blockchain, you've probably been frustrated by
either non existant or opaque error codes. Anchor attempts to address this by
providing the `#[error]` attribute, which can be used to create typed Errors with
descriptive messages that automatically propagate to the client.

## Defining a Program

For example,

```rust
use anchor_lang::prelude::*;

#[program]
mod errors {
    use super::*;
    pub fn hello(_ctx: Context<Hello>) -> Result<()> {
        Err(ErrorCode::Hello.into())
    }
}

#[derive(Accounts)]
pub struct Hello {}

#[error]
pub enum ErrorCode {
    #[msg("This is an error message clients will automatically display")]
    Hello,
}
```

Observe the [#[error]](https://docs.rs/anchor-lang/latest/anchor_lang/attr.error.html) attribute on the `ErrorCode` enum. This macro generates two types: an `Error` and a `Result`, both of which can be used when returning from your program.

To use the `Error`, you can simply use the user defined `ErrorCode` with Rust's [From](https://doc.rust-lang.org/std/convert/trait.From.html) trait. If you're unfamiliar with `From`, no worries. Just know that you need to either call
`.into()` when using your `ErrorCode`. Or use Rust's `?` operator, when returning an error.
Both of these will automatically convert *into* the correct `Error`.

::: details
What's the deal with this From stuff? Well, because the Solana runtime expects a [ProgramError](https://docs.rs/solana-program/1.5.5/solana_program/program_error/enum.ProgramError.html) in the return value. The framework needs to wrap the user defined error code into a
`ProgramError::Code` variant, before returning. The alternative would be to use the
`ProgramError` directly.
:::

## Using the Client

When using the client, we get the error message.

```javascript
try {
  const tx = await program.rpc.hello();
  assert.ok(false);
} catch (err) {
  const errMsg = "This is an error message clients will automatically display";
  assert.equal(err.toString(), errMsg);
}
```

It's that easy. :)

To run the full example, go [here](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-4).
