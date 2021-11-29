<div align="center">
  <img height="170x" src="https://media.discordapp.net/attachments/813444514949103658/890278520553603092/export.png?width=746&height=746" />

  <h1>Anchor</h1>

  <p>
    <strong>Solana Sealevel Framework</strong>
  </p>

  <p>
    <a href="https://github.com/project-serum/anchor/actions"><img alt="Build Status" src="https://github.com/project-serum/anchor/actions/workflows/tests.yaml/badge.svg" /></a>
    <a href="https://project-serum.github.io/anchor/"><img alt="Tutorials" src="https://img.shields.io/badge/docs-tutorials-blueviolet" /></a>
    <a href="https://discord.com/channels/889577356681945098"><img alt="Discord Chat" src="https://img.shields.io/discord/889577356681945098?color=blueviolet" /></a>
    <a href="https://opensource.org/licenses/Apache-2.0"><img alt="License" src="https://img.shields.io/github/license/project-serum/anchor?color=blueviolet" /></a>
  </p>
</div>

Anchor is a framework for Solana's [Sealevel](https://medium.com/solana-labs/sealevel-parallel-processing-thousands-of-smart-contracts-d814b378192) runtime providing several convenient developer tools for writing smart contracts.

- Rust eDSL for writing Solana programs
- [IDL](https://en.wikipedia.org/wiki/Interface_description_language) specification
- TypeScript package for generating clients from IDL
- CLI and workspace management for developing complete applications

If you're familiar with developing in Ethereum's [Solidity](https://docs.soliditylang.org/en/v0.7.4/), [Truffle](https://www.trufflesuite.com/), [web3.js](https://github.com/ethereum/web3.js), then the experience will be familiar. Although the DSL syntax and semantics are targeted at Solana, the high level flow of writing RPC request handlers, emitting an IDL, and generating clients from IDL is the same.

## Getting Started

For a quickstart guide and in depth tutorials, see the guided [documentation](https://project-serum.github.io/anchor/getting-started/introduction.html).
To jump straight to examples, go [here](https://github.com/project-serum/anchor/tree/master/examples). For the latest Rust and TypeScript API documentation, see [docs.rs](https://docs.rs/anchor-lang) and the [typedoc](https://project-serum.github.io/anchor/ts/index.html).

## Packages

| Package | Description | Version | Docs |
| :-- | :-- | :--| :-- |
| `anchor-lang` | Rust primitives for writing programs on Solana | [![Crates.io](https://img.shields.io/crates/v/anchor-lang?color=blue)](https://crates.io/crates/anchor-lang) | [![Docs.rs](https://docs.rs/anchor-lang/badge.svg)](https://docs.rs/anchor-lang) |
| `anchor-spl` | CPI clients for SPL programs on Solana | ![crates](https://img.shields.io/crates/v/anchor-spl?color=blue) | [![Docs.rs](https://docs.rs/anchor-spl/badge.svg)](https://docs.rs/anchor-spl) |
| `anchor-client` | Rust client for Anchor programs | ![crates](https://img.shields.io/crates/v/anchor-client?color=blue) | [![Docs.rs](https://docs.rs/anchor-client/badge.svg)](https://docs.rs/anchor-client) |
| `@project-serum/anchor` | TypeScript client for Anchor programs | [![npm](https://img.shields.io/npm/v/@project-serum/anchor.svg?color=blue)](https://www.npmjs.com/package/@project-serum/anchor) | [![Docs](https://img.shields.io/badge/docs-typedoc-blue)](https://project-serum.github.io/anchor/ts/index.html) |
| `@project-serum/anchor-cli` | CLI to support building and managing an Anchor workspace | [![npm](https://img.shields.io/npm/v/@project-serum/anchor-cli.svg?color=blue)](https://www.npmjs.com/package/@project-serum/anchor-cli) | [![Docs](https://img.shields.io/badge/docs-typedoc-blue)](https://project-serum.github.io/anchor/cli/commands.html) |

## Note

* **Anchor is in active development, so all APIs are subject to change.**
* **This code is unaudited. Use at your own risk.**

## Examples

Here's a counter program, where only the designated `authority`
can increment the count.

```rust
use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
mod counter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, start: u64) -> ProgramResult {
        let counter = &mut ctx.accounts.counter;
        counter.authority = *ctx.accounts.authority.key;
        counter.count = start;
        Ok(())
    }

    pub fn increment(ctx: Context<Increment>) -> ProgramResult {
        let counter = &mut ctx.accounts.counter;
        counter.count += 1;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 48)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[account]
pub struct Counter {
    pub authority: Pubkey,
    pub count: u64,
}
```

For more, see the [examples](https://github.com/project-serum/anchor/tree/master/examples)
and [tests](https://github.com/project-serum/anchor/tree/master/tests) directories.

## Contribution

Thank you for your interest in contributing to Anchor! All contributions are welcome no
matter how big or small. This includes (but is not limited to) filing issues,
adding documentation, fixing bugs, creating examples, and implementing features.

If you'd like to contribute, please claim an issue by commenting, forking, and
opening a pull request, even if empty. This allows the maintainers to track who
is working on what issue as to not overlap work. If you're looking to get started,
check out [good first issues](https://github.com/project-serum/anchor/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22)
or issues where [help is wanted](https://github.com/project-serum/anchor/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22).
For simple documentation changes, feel free to just open a pull request.

If you're considering larger changes or self motivated features, please file an issue
and engage with the maintainers in [Discord](https://discord.gg/sxy4zxBckh).

## License

Anchor is licensed under [Apache 2.0](./LICENSE).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Anchor by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.
