# Errors

If you've ever programmed on a blockchain, you've probably been frustrated by
either non existent or opaque error codes. Anchor attempts to address this by
providing the `#[error_code]` attribute, which can be used to create typed Errors with
descriptive messages that automatically propagate to the client.

## Defining a Program

For example,

```rust
use anchor_lang::prelude::*;

#[program]
mod errors {
    use super::*;
    pub fn hello(_ctx: Context<Hello>) -> Result<()> {
        Err(error!(ErrorCode::Hello))
    }
}

#[derive(Accounts)]
pub struct Hello {}

#[error_code]
pub enum ErrorCode {
    #[msg("This is an error message clients will automatically display")]
    Hello,
}
```

Observe the [#[error_code]](https://docs.rs/anchor-lang/latest/anchor_lang/attr.error_code.html) attribute on the `ErrorCode` enum.
This macro generates internal anchor code that helps anchor turn the error code into an error and display it properly.

To create an error, use the [`error!`](https://docs.rs/anchor-lang/latest/anchor_lang/prelude/macro.error.html) macro together with an error code. This macro creates an [`AnchorError`](https://docs.rs/anchor-lang/latest/anchor_lang/error/struct.AnchorError.html) that includes helpful information like the file and line the error was created in.

To make writing errors even easier, anchor also provides the [`err!`](https://docs.rs/anchor-lang/latest/anchor_lang/prelude/macro.err.html) and the [`require!`](https://docs.rs/anchor-lang/latest/anchor_lang/prelude/macro.require.html) macros.

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
