# Anchor ⚓

[![Build Status](https://travis-ci.com/project-serum/anchor.svg?branch=master)](https://travis-ci.com/project-serum/anchor)
[![Docs.rs](https://docs.rs/anchor-lang/badge.svg)](https://docs.rs/anchor-lang)
[![Docs](https://img.shields.io/badge/docs-tutorials-orange)](https://project-serum.github.io/anchor/)
[![Chat](https://img.shields.io/discord/739225212658122886?color=blueviolet)](https://discord.com/channels/739225212658122886)
[![License](https://img.shields.io/github/license/project-serum/anchor?color=blue)](https://opensource.org/licenses/Apache-2.0)

Anchor is a framework for Solana's [Sealevel](https://medium.com/solana-labs/sealevel-parallel-processing-thousands-of-smart-contracts-d814b378192) runtime providing several convenient developer tools.

- Rust eDSL for writing Solana programs
- [IDL](https://en.wikipedia.org/wiki/Interface_description_language) specification
- TypeScript package for generating clients from IDL
- CLI and workspace management for developing complete applications

If you're familiar with developing in Ethereum's [Solidity](https://docs.soliditylang.org/en/v0.7.4/), [Truffle](https://www.trufflesuite.com/), [web3.js](https://github.com/ethereum/web3.js) or Parity's [Ink!](https://github.com/paritytech/ink), then the experience will be familiar. Although the DSL syntax and semantics are targeted at Solana, the high level flow of writing RPC request handlers, emitting an IDL, and generating clients from IDL is the same.

## Getting Started

For a quickstart guide and in depth tutorials, see the guided [documentation](https://project-serum.github.io/anchor/getting-started/introduction.html).
To jump straight to examples, go [here](https://github.com/project-serum/anchor/tree/master/examples). For the latest Rust API documentation, see [docs.rs](https://docs.rs/anchor-lang).

## Note

* **Anchor is in active development, so all APIs are subject to change.**
* **This code is unaudited. Use at your own risk.**

## Examples

Build stateful programs on Solana by defining a state struct with associated
methods. Here's a classic counter example, where only the designated `authority`
can increment the count.

```rust
#[program]
mod counter {

    #[state]
    pub struct Counter {
      authority: Pubkey,
      count: u64,
    }

    pub fn new(ctx: Context<Auth>) -> Result<Self> {
        Ok(Self {
            auth: *ctx.accounts.authority.key
        })
    }

    pub fn increment(&mut self, ctx: Context<Auth>) -> Result<()> {
        if &self.authority != ctx.accounts.authority.key {
            return Err(ErrorCode::Unauthorized.into());
        }

        self.count += 1;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Auth<'info> {
    #[account(signer)]
    authority: AccountInfo<'info>,
}

#[error]
pub enum ErrorCode {
    #[msg("You are not authorized to perform this action.")]
    Unauthorized,
}
```

Additionally, one can utilize the full power of Solana's parallel execution model by
keeping the program stateless and working with accounts directly. The above example
can be rewritten as follows.

```rust
use anchor::prelude::*;

// Define instruction handlers.

#[program]
mod counter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, authority: Pubkey) -> ProgramResult {
        let counter = &mut ctx.accounts.counter;

        counter.authority = authority;
        counter.count = 0;

        Ok(())
    }

    pub fn increment(ctx: Context<Update>) -> ProgramResult {
        let counter = &mut ctx.accounts.counter;

        counter += 1;

        Ok(())
    }
}

// Define accounts for each handler.

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init)]
    pub counter: ProgramAccount<'info, Counter>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut, has_one = authority)]
    pub counter: ProgramAccount<'info, Counter>,
    #[account(signer)]
    pub authority: AccountInfo<'info>,
}

// Define program owned accounts.

#[account]
pub struct Counter {
    pub authority: Pubkey,
    pub count: u64,
}
```

Due to the fact that account sizes on Solana are fixed, some combination of
the above is often required. For example, one can store store global state
associated with the entire program in the `#[state]` struct and local
state assocated with each user in individual `#[account]` structs.

## Accounts attribute syntax.

There are several inert attributes (attributes that are consumed only for the
purposes of the Accounts macro) that can be specified on a struct deriving `Accounts`.

| Attribute | Where Applicable | Description |
|:--|:--|:--|
| `#[account(signer)]` | On raw `AccountInfo` structs. | Checks the given account signed the transaction. |
| `#[account(mut)]` | On `AccountInfo`, `ProgramAccount` or `CpiAccount` structs. | Marks the account as mutable and persists the state transition. |
| `#[account(init)]` | On `ProgramAccount` structs. | Marks the account as being initialized, skipping the account discriminator check. |
| `#[account(belongs_to = <target>)]` | On `ProgramAccount` or `CpiAccount` structs | Checks the `target` field on the account matches the `target` field in the struct deriving `Accounts`. |
| `#[account(has_one = <target>)]` | On `ProgramAccount` or `CpiAccount` structs | Semantically different, but otherwise the same as `belongs_to`. |
| `#[account(seeds = [<seeds>])]` | On `AccountInfo` structs | Seeds for the program derived address an `AccountInfo` struct represents. |
| `#[account(owner = program \| skip)]` | On `AccountInfo` structs | Checks the owner of the account is the current program or skips the check. |
| `#[account("<literal>")]` | On any type deriving `Accounts` | Executes the given code literal as a constraint. The literal should evaluate to a boolean. |
| `#[account(rent_exempt = <skip>)]` | On `AccountInfo` or `ProgramAccount` structs | Optional attribute to skip the rent exemption check. By default, all accounts marked with `#[account(init)]` will be rent exempt, and so this should rarely (if ever) be used. Similarly, omitting `= skip` will mark the account rent exempt. |

## License

Anchor is licensed under [Apache 2.0](./LICENSE).
