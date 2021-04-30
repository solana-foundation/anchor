# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Note:** Version 0 of Semantic Versioning is handled differently from version 1 and above.
The minor version will be incremented upon a breaking change and the patch version will be
incremented for features.

## [Unreleased]

## [0.4.5] - 2021-04-29

* spl: Add serum DEX CPI client ([#224](https://github.com/project-serum/anchor/pull/224)).

## [0.4.4] - 2021-04-18

## Features

* lang: Allows one to specify multiple `with` targets when creating associated acconts ([#197](https://github.com/project-serum/anchor/pull/197)).
* lang, ts: Add array support ([#202](https://github.com/project-serum/anchor/pull/202)).
* lang: Zero copy deserialization for accounts ([#202](https://github.com/project-serum/anchor/pull/202), [#206](https://github.com/project-serum/anchor/pull/206)).
* lang, spl, cli, client: Upgrade solana toolchain to 1.6.6 ([#210](https://github.com/project-serum/anchor/pull/210)).

## [0.4.3] - 2021-04-13

## Features

* lang: CPI clients for program state instructions ([#43](https://github.com/project-serum/anchor/pull/43)).
* lang: Add `#[account(owner = <program>)]` constraint ([#178](https://github.com/project-serum/anchor/pull/178)).
* lang, cli, ts: Add `#[account(associated = <target>)]` and `#[associated]` attributes for creating associated program accounts within programs. The TypeScript package can fetch these accounts with a new `<program>.account.<account-name>.associated` (and `associatedAddress`) method ([#186](https://github.com/project-serum/anchor/pull/186)).

## Fixes

* lang: Unused `#[account]`s are now parsed into the IDL correctly ([#177](https://github.com/project-serum/anchor/pull/177)).

## [0.4.2] - 2021-04-10

## Features

* cli: Fund Anchor.toml configured wallet when testing ([#164](https://github.com/project-serum/anchor/pull/164)).
* spl: Add initialize_account instruction for spl tokens ([#166](https://github.com/project-serum/anchor/pull/166)).

## [0.4.1] - 2021-04-06

* cli: Version verifiable docker builder ([#145](https://github.com/project-serum/anchor/pull/145)).

## [0.4.0] - 2021-04-04

## Features

* cli: Specify test files to run ([#118](https://github.com/project-serum/anchor/pull/118)).
* lang: Allow overriding the `#[state]` account's size ([#121](https://github.com/project-serum/anchor/pull/121)).
* lang, client, ts: Add event emission and subscriptions ([#89](https://github.com/project-serum/anchor/pull/89)).
* lang/account: Allow namespacing account discriminators ([#128](https://github.com/project-serum/anchor/pull/128)).
* cli: TypeScript migrations ([#132](https://github.com/project-serum/anchor/pull/132)).
* lang: Add `#[account(executable)]` attribute ([#140](https://github.com/project-serum/anchor/pull/140)).

## Breaking Changes

* client: Replace url str with `Cluster` struct when constructing clients ([#89](https://github.com/project-serum/anchor/pull/89)).
* lang: Changes the account discriminator of `IdlAccount` to be namespaced by `"internal"` ([#128](https://github.com/project-serum/anchor/pull/128)).
* lang, spl, cli: Upgrade solana toolchain to 1.6.3, a major version upgrade even though only the minor version is incremented. This allows for the removal of `-#![feature(proc_macro_hygiene)]`. ([#139](https://github.com/project-serum/anchor/pull/139)).

## [0.3.0] - 2021-03-12

## Features

* ts: Allow preloading instructions for state rpc transactions ([cf9c84](https://github.com/project-serum/anchor/commit/cf9c847e4144989b5bc1936149d171e90204777b)).
* ts: Export sighash coder function ([734c75](https://github.com/project-serum/anchor/commit/734c751882f43beec7ea3f0f4d988b502e3f24e4)).
* cli: Specify programs to embed into local validator genesis via Anchor.toml while testing ([b3803a](https://github.com/project-serum/anchor/commit/b3803aec03fbbae1a794c9aa6a789e6cb58fda99)).
* cli: Allow skipping the creation of a local validator when testing against localnet ([#93](https://github.com/project-serum/anchor/pull/93)).
* cli: Adds support for tests with Typescript ([#94](https://github.com/project-serum/anchor/pull/94)).
* cli: Deterministic and verifiable builds ([#100](https://github.com/project-serum/anchor/pull/100)).
* cli, lang: Add write buffers for IDL upgrades ([#107](https://github.com/project-serum/anchor/pull/107)).

## Breaking Changes

* lang: Removes `IdlInstruction::Clear` ([#107](https://github.com/project-serum/anchor/pull/107)).

## Fixes

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
