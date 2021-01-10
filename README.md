# Anchor ⚓

[![Build Status](https://travis-ci.com/project-serum/anchor.svg?branch=master)](https://travis-ci.com/project-serum/anchor)
[![Docs](https://img.shields.io/badge/docs-tutorials-orange)](https://project-serum.github.io/anchor/)
[![Chat](https://img.shields.io/discord/739225212658122886?color=blueviolet)](https://discord.com/channels/739225212658122886)
[![License](https://img.shields.io/github/license/project-serum/anchor?color=blue)](https://opensource.org/licenses/Apache-2.0)

Anchor is a framework for Solana's [Sealevel](https://medium.com/solana-labs/sealevel-parallel-processing-thousands-of-smart-contracts-d814b378192) runtime providing several convenient developer tools.

- Rust DSL for writing Solana programs
- [IDL](https://en.wikipedia.org/wiki/Interface_description_language) specification
- TypeScript package for generating clients from IDL
- CLI and workspace management for developing complete applications

If you're familiar with developing in Ethereum's [Solidity](https://docs.soliditylang.org/en/v0.7.4/), [Truffle](https://www.trufflesuite.com/), [web3.js](https://github.com/ethereum/web3.js) or Parity's [Ink!](https://github.com/paritytech/ink), then the experience will be familiar. Although the DSL syntax and semantics are targeted at Solana, the high level flow of writing RPC request handlers, emitting an IDL, and generating clients from IDL is the same.

## Getting Started

For a quickstart guide and in depth tutorials, see the guided [documentation](https://project-serum.github.io/anchor/getting-started/introduction.html).
To jump straight to examples, go [here](https://github.com/project-serum/anchor/tree/master/examples/tutorial).

## Note

* **Anchor is in active development, so all APIs are subject to change.**
* **This code is unaudited. Use at your own risk.**

## Example

```Rust
use anchor::prelude::*;

// Define the program's RPC handlers.

#[program]
mod basic_1 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, authority: Pubkey) -> ProgramResult {
        let my_account = &mut ctx.accounts.my_account;
        my_account.authority = authority;
        Ok(())
    }

    #[access_control(not_zero(data))]
    pub fn update(ctx: Context<Update>, data: u64) -> ProgramResult {
        let my_account = &mut ctx.accounts.my_account;
        my_account.data = data;
        Ok(())
    }
}

// Define the validated accounts for each handler.

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init)]
    pub my_account: ProgramAccount<'info, MyAccount>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(signer)]
    pub authority: AccountInfo<'info>,
    #[account(mut, "&my_account.authority == authority.key")]
    pub my_account: ProgramAccount<'info, MyAccount>,
}

// Define program owned accounts.

#[account]
pub struct MyAccount {
    pub authority: Pubkey,
    pub data: u64,
}

// Define auxiliary access control checks.

fn not_zero(data: u64) -> ProgramResult {
    if data == 0 {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}
```

## Accounts attribute syntax.

There are several inert attributes (attributes that are consumed only for the
purposes of the Accounts macro) that can be specified on a struct deriving `Accounts`.

| Attribute | Where Applicable | Description |
|:--|:--|:--|
| `#[account(signer)]` | On raw `AccountInfo` structs. | Checks the given account signed the transaction. |
| `#[account(mut)]` | On `ProgramAccount` structs. | Marks the account as mutable and persists the state transition. |
| `#[account(init)]` | On `ProgramAccount` structs. | Marks the account as being initialized, skipping the account discriminator check. |
| `#[account(belongs_to = <target>)]` | On `ProgramAccount` structs | Checks the `target` field on the account matches the `target` field in the accounts array. |
| `#[account(owner = program \| skip)]` | On `ProgramAccount` and `AccountInfo` structs | Checks the owner of the account is the current program or skips the check. |
| `#[account("<literal>")]` | On `ProgramAccount` structs | Executes the given code literal as a constraint. The literal should evaluate to a boolean. |

## License

Anchor is licensed under [Apache 2.0](./LICENSE).
