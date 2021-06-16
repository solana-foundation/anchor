# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Note:** Version 0 of Semantic Versioning is handled differently from version 1 and above.
The minor version will be incremented upon a breaking change and the patch version will be
incremented for features.

## [Unreleased]

## [0.9.0] - 2021-06-15

### Features

* lang: Instruction data is now available to accounts constraints ([#386](https://github.com/project-serum/anchor/pull/386)).
* lang: Initialize program derived addresses with accounts constraints ([#386](https://github.com/project-serum/anchor/pull/386)).

### Breaking Changes

* lang: Event field names in IDLs are now mixed case. ([#379](https://github.com/project-serum/anchor/pull/379)).
* lang: Accounts trait now accepts an additional `&[u8]` parameter ([#386](https://github.com/project-serum/anchor/pull/386)).

## [0.8.0] - 2021-06-10

### Features

* cli: Add `--program-name` option for build command to build a single program at a time ([#362](https://github.com/project-serum/anchor/pull/362)).
* cli, client: Parse custom cluster urls from str ([#369](https://github.com/project-serum/anchor/pull/369)).
* cli, client, lang: Update solana toolchain to v1.7.1 ([#368](https://github.com/project-serum/anchor/pull/369)).
* ts: Instruction decoding and formatting ([#372](https://github.com/project-serum/anchor/pull/372)).
* lang: Add `#[account(close = <destination>)]` constraint for closing accounts and sending the rent exemption lamports to a specified destination account ([#371](https://github.com/project-serum/anchor/pull/371)).

### Fixes

* lang: Allows one to use `remaining_accounts` with `CpiContext` by implementing the `ToAccountMetas` trait on `CpiContext` ([#351](https://github.com/project-serum/anchor/pull/351/files)).

### Breaking

* lang, ts: Framework defined error codes are introduced, reserving error codes 0-300 for Anchor, and 300 and up for user defined error codes ([#354](https://github.com/project-serum/anchor/pull/354)).

## [0.7.0] - 2021-05-31

### Features

* cli: Add global options for override Anchor.toml values ([#313](https://github.com/project-serum/anchor/pull/313)).
* spl: Add `SetAuthority` instruction ([#307](https://github.com/project-serum/anchor/pull/307/files)).
* spl: Add init and close open orders instructions ([#245](https://github.com/project-serum/anchor/pull/245)).
* lang: `constraint = <expression>` added as a replacement for (the now deprecated) string literal constraints ([#341](https://github.com/project-serum/anchor/pull/341)).
* lang: Span information is now preserved, providing informative compiler error messages ([#341](https://github.com/project-serum/anchor/pull/341)).
* ts: Address metadata is now optional for `anchor.workspace` clients ([#310](https://github.com/project-serum/anchor/pull/310)).

### Breaking Changes

* ts: Retrieving deserialized accounts from the `<program>.account.<my-account>` and `<program>.state` namespaces now require explicitly invoking the `fetch` API. For example, `program.account.myAccount(<adddress>)` and `program.state()` is now `program.account.myAccount.fetch(<address>)` and `program.state.fetch()` ([#322](https://github.com/project-serum/anchor/pull/322)).
* lang: `#[account(associated)]` now requires `init` to be provided to create an associated account. If not provided, then the address will be assumed to exist, and a constraint will be added to ensure the correctness of the address ([#318](https://github.com/project-serum/anchor/pull/318)).
* lang, ts: Change account discriminator pre-image of the `#[state]` account discriminator to be namespaced by "state:" ([#320](https://github.com/project-serum/anchor/pull/320)).
* lang, ts: Change domain delimiters for the pre-image of the instruciton sighash to be a single colon `:` to be consistent with accounts ([#321](https://github.com/project-serum/anchor/pull/321)).
* lang: Associated constraints no longer automatically implement `mut` ([#341](https://github.com/project-serum/anchor/pull/341)).
* lang: Associated `space` constraints must now be literal integers instead of literal strings ([#341](https://github.com/project-serum/anchor/pull/341)).

## [0.6.0] - 2021-05-23

### Features

* ts: Add `program.simulate` namespace ([#266](https://github.com/project-serum/anchor/pull/266)).
* ts: Introduce `Address` type, allowing one to use Base 58 encoded strings in public APIs ([#304](https://github.com/project-serum/anchor/pull/304)).
* ts: Replace deprecated `web3.Account` with `web3.Signer` in public APIs ([#296](https://github.com/project-serum/anchor/pull/296)).
* ts: Generated `anchor.workspace` clients can now be customized per network with `[cluster.<slug>]` in the Anchor.toml ([#308](https://github.com/project-serum/anchor/pull/308)).
* cli: Add yarn flag to test command ([#267](https://github.com/project-serum/anchor/pull/267)).
* cli: Add `--skip-build` flag to test command ([301](https://github.com/project-serum/anchor/pull/301)).
* cli: Add `anchor shell` command to spawn a node shell populated with an Anchor.toml based environment ([#303](https://github.com/project-serum/anchor/pull/303)).

### Breaking Changes

* cli: The Anchor.toml's `wallet` and `cluster` settings must now be under the `[provider]` table ([#305](https://github.com/project-serum/anchor/pull/305)).
* ts: Event coder `decode` API changed to decode strings directly instead of buffers ([#292](https://github.com/project-serum/anchor/pull/292)).
* ts: Event coder `encode` API removed ([#292](https://github.com/project-serum/anchor/pull/292)).

## [0.5.0] - 2021-05-07

### Features

* client: Adds support for state instructions ([#248](https://github.com/project-serum/anchor/pull/248)).
* lang: Add `anchor-debug` feature flag for logging ([#253](https://github.com/project-serum/anchor/pull/253)).
* ts: Add support for u16 ([#255](https://github.com/project-serum/anchor/pull/255)).

### Breaking Changes

* client: Renames `RequestBuilder::new` to `RequestBuilder::from` ([#248](https://github.com/project-serum/anchor/pull/248)).
* lang: Renames the generated `instruction::state::Ctor` struct to `instruction::state::New` ([#248](https://github.com/project-serum/anchor/pull/248)).

## [0.4.5] - 2021-04-29

* spl: Add serum DEX CPI client ([#224](https://github.com/project-serum/anchor/pull/224)).

## [0.4.4] - 2021-04-18

### Features

* lang: Allows one to specify multiple `with` targets when creating associated acconts ([#197](https://github.com/project-serum/anchor/pull/197)).
* lang, ts: Add array support ([#202](https://github.com/project-serum/anchor/pull/202)).
* lang: Zero copy deserialization for accounts ([#202](https://github.com/project-serum/anchor/pull/202), [#206](https://github.com/project-serum/anchor/pull/206)).
* lang, spl, cli, client: Upgrade solana toolchain to 1.6.6 ([#210](https://github.com/project-serum/anchor/pull/210)).

## [0.4.3] - 2021-04-13

### Features

* lang: CPI clients for program state instructions ([#43](https://github.com/project-serum/anchor/pull/43)).
* lang: Add `#[account(owner = <program>)]` constraint ([#178](https://github.com/project-serum/anchor/pull/178)).
* lang, cli, ts: Add `#[account(associated = <target>)]` and `#[associated]` attributes for creating associated program accounts within programs. The TypeScript package can fetch these accounts with a new `<program>.account.<account-name>.associated` (and `associatedAddress`) method ([#186](https://github.com/project-serum/anchor/pull/186)).

### Fixes

* lang: Unused `#[account]`s are now parsed into the IDL correctly ([#177](https://github.com/project-serum/anchor/pull/177)).

## [0.4.2] - 2021-04-10

### Features

* cli: Fund Anchor.toml configured wallet when testing ([#164](https://github.com/project-serum/anchor/pull/164)).
* spl: Add initialize_account instruction for spl tokens ([#166](https://github.com/project-serum/anchor/pull/166)).

## [0.4.1] - 2021-04-06

* cli: Version verifiable docker builder ([#145](https://github.com/project-serum/anchor/pull/145)).

## [0.4.0] - 2021-04-04

### Features

* cli: Specify test files to run ([#118](https://github.com/project-serum/anchor/pull/118)).
* lang: Allow overriding the `#[state]` account's size ([#121](https://github.com/project-serum/anchor/pull/121)).
* lang, client, ts: Add event emission and subscriptions ([#89](https://github.com/project-serum/anchor/pull/89)).
* lang/account: Allow namespacing account discriminators ([#128](https://github.com/project-serum/anchor/pull/128)).
* cli: TypeScript migrations ([#132](https://github.com/project-serum/anchor/pull/132)).
* lang: Add `#[account(executable)]` attribute ([#140](https://github.com/project-serum/anchor/pull/140)).

### Breaking Changes

* client: Replace url str with `Cluster` struct when constructing clients ([#89](https://github.com/project-serum/anchor/pull/89)).
* lang: Changes the account discriminator of `IdlAccount` to be namespaced by `"internal"` ([#128](https://github.com/project-serum/anchor/pull/128)).
* lang, spl, cli: Upgrade solana toolchain to 1.6.3, a major version upgrade even though only the minor version is incremented. This allows for the removal of `-#![feature(proc_macro_hygiene)]`. ([#139](https://github.com/project-serum/anchor/pull/139)).

## [0.3.0] - 2021-03-12

### Features

* ts: Allow preloading instructions for state rpc transactions ([cf9c84](https://github.com/project-serum/anchor/commit/cf9c847e4144989b5bc1936149d171e90204777b)).
* ts: Export sighash coder function ([734c75](https://github.com/project-serum/anchor/commit/734c751882f43beec7ea3f0f4d988b502e3f24e4)).
* cli: Specify programs to embed into local validator genesis via Anchor.toml while testing ([b3803a](https://github.com/project-serum/anchor/commit/b3803aec03fbbae1a794c9aa6a789e6cb58fda99)).
* cli: Allow skipping the creation of a local validator when testing against localnet ([#93](https://github.com/project-serum/anchor/pull/93)).
* cli: Adds support for tests with Typescript ([#94](https://github.com/project-serum/anchor/pull/94)).
* cli: Deterministic and verifiable builds ([#100](https://github.com/project-serum/anchor/pull/100)).
* cli, lang: Add write buffers for IDL upgrades ([#107](https://github.com/project-serum/anchor/pull/107)).

## Breaking Changes

* lang: Removes `IdlInstruction::Clear` ([#107](https://github.com/project-serum/anchor/pull/107)).

### Fixes

* cli: Propagates mocha test exit status on error ([79b791](https://github.com/project-serum/anchor/commit/79b791ffa85ffae5b6163fa853562aa568650f21)).

## [0.2.1] - 2021-02-11

### Features

* cli: Embed workspace programs into local validator genesis when testing ([733ec3](https://github.com/project-serum/anchor/commit/733ec300b0308e7d007873b0975585d836333fd4)).
* cli: Stream program logs to `.anchor/program-logs` directory when testing ([ce5ca7](https://github.com/project-serum/anchor/commit/ce5ca7ddab6e0fd579deddcd02094b3f798bbe6a)).
* spl: Add shared memory api [(d92cb1)](https://github.com/project-serum/anchor/commit/d92cb1516b78696d1257e41d0c5ac6821716300e).
* lang/attribute/access-control: Allow specifying multiple modifier functions ([845df6](https://github.com/project-serum/anchor/commit/845df6d1960bb544fa0f2e3331ec79cc804edeb6)).
* lang/syn: Allow state structs that don't have a ctor or impl block (just trait implementations) ([a78000](https://github.com/project-serum/anchor/commit/a7800026833d64579e5b19c90d724ecc20d2a455)).
* ts: Add instruction method to state namespace ([627c27](https://github.com/project-serum/anchor/commit/627c275e9cdf3dafafcf44473ba8146cc7979d44)).
* lang/syn, ts: Add support for u128 and i128 ([#83](https://github.com/project-serum/anchor/pull/83)).

## [0.2.0] - 2021-02-08

### Features

* lang: Adds the ability to create and use CPI program interfaces ([#66](https://github.com/project-serum/anchor/pull/66/files?file-filters%5B%5D=)).

### Breaking Changes

* lang, client, ts: Migrate from rust enum based method dispatch to a variant of sighash ([#64](https://github.com/project-serum/anchor/pull/64)).

## [0.1.0] - 2021-01-31

Initial release.

### Includes

* lang: `anchor-lang` crate providing a Rust eDSL for Solana.
* lang/attribute/access-control: Internal attribute macro for function modifiers.
* lang/attribute/account: Internal attribute macro for defining Anchor accounts.
* lang/attribute/error: Internal attribute macro for defining Anchor program errors.
* lang/attribute/program: Internal attribute macro for defining an Anchor program.
* lang/attribute/state: Internal attribute macro for defining an Anchor program state struct.
* lang/derive/accounts: Internal derive macro for defining deserialized account structs.
* lang/syn: Internal crate for parsing the Anchor eDSL, generating code, and an IDL.
* spl: `anchor-spl` crate providing CPI clients for Anchor programs.
* client: `anchor-client` crate providing Rust clients for Anchor programs.
* ts: `@project-serum/anchor` package for generating TypeScript clients.
* cli: Command line interface for managing Anchor programs.
