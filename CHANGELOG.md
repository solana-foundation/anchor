# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Note:** Version 0 of Semantic Versioning is handled differently from version 1 and above.
The minor version will be incremented upon a breaking change and the patch version will be
incremented for features.

## [Unreleased]

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
