# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Note:** Version 0 of Semantic Versioning is handled differently from version 1 and above.
The minor version will be incremented upon a breaking change and the patch version will be
incremented for features.

## [Unreleased]

## [0.19.0] - 2021-12-08

### Fixes

* lang: Add `deprecated` attribute to `ProgramAccount` ([#1014](https://github.com/project-serum/anchor/pull/1014)).
* cli: Add version number from programs `Cargo.toml` into extracted IDL ([#1061](https://github.com/project-serum/anchor/pull/1061)).
* lang: Add `deprecated` attribute to `Loader`([#1078](https://github.com/project-serum/anchor/pull/1078)).
* lang: the `init_if_needed` attribute now checks that given attributes (e.g. space, owner, token::authority etc.) are validated even when init is not needed ([#1096](https://github.com/project-serum/anchor/pull/1096)).

### Features

* lang: Add `ErrorCode::AccountNotInitialized` error to separate the situation when the account has the wrong owner from when it does not exist (#[1024](https://github.com/project-serum/anchor/pull/1024)).
* lang: Called instructions now log their name by default. This can be turned off with the `no-log-ix-name` flag ([#1057](https://github.com/project-serum/anchor/pull/1057)).
* lang: `ProgramData` and `UpgradableLoaderState` can now be passed into `Account` as generics. see [UpgradeableLoaderState](https://docs.rs/solana-program/latest/solana_program/bpf_loader_upgradeable/enum.UpgradeableLoaderState.html). `UpgradableLoaderState` can also be matched on to get `ProgramData`, but when `ProgramData` is used instead, anchor does the serialization and checking that it is actually program data for you  ([#1095](https://github.com/project-serum/anchor/pull/1095)).
* ts: Add better error msgs in the ts client if something wrong (i.e. not a pubkey or a string) is passed in as an account in an instruction accounts object ([#1098](https://github.com/project-serum/anchor/pull/1098)).
* ts: Add inputs `postInstructions` and `preInstructions` as a replacement for (the now deprecated) `instructions` ([#1007](https://github.com/project-serum/anchor/pull/1007)).
* ts: Add `getAccountInfo` helper method to account namespace/client ([#1084](https://github.com/project-serum/anchor/pull/1084)).

### Breaking

* lang, ts: Error codes have been mapped to new numbers to allow for more errors per namespace ([#1096](https://github.com/project-serum/anchor/pull/1096)).

## [0.18.2] - 2021-11-14

* cli: Replace global JavaScript dependency installs with local.

### Features

* lang: Add `SystemAccount<'info>` account type for generic wallet addresses or accounts owned by the system program ([#954](https://github.com/project-serum/anchor/pull/954))

### Fixes

* cli: fix dns in NODE_OPTIONS ([#928](https://github.com/project-serum/anchor/pull/928)).
* cli: output TypeScript IDL in `idl parse` subcommand ([#941](https://github.com/project-serum/anchor/pull/941)).
* cli: Add fields `os` and `cpu` to npm package `@project-serum/anchor-cli` ([#976](https://github.com/project-serum/anchor/pull/976)).
* cli: Allow specify output directory for TypeScript IDL ([#940](https://github.com/project-serum/anchor/pull/940)).

### Breaking

* spl: Move permissioned markets into dex repository ([#962](https://github.com/project-serum/anchor/pull/962)).

## [0.18.0] - 2021-10-24

### Features

* cli: Add support for configuration options for `solana-test-validator` in Anchor.toml ([#834](https://github.com/project-serum/anchor/pull/834)).
* cli: `target/types` directory now created on build to store a TypeScript types file for each program's IDL ([#795](https://github.com/project-serum/anchor/pull/795)).
* ts: `Program<T>` can now be typed with an IDL type ([#795](https://github.com/project-serum/anchor/pull/795)).
* lang: Add `mint::freeze_authority` keyword for mint initialization within `#[derive(Accounts)]` ([#835](https://github.com/project-serum/anchor/pull/835)).
* lang: Add `AccountLoader` type for `zero_copy` accounts with support for CPI ([#792](https://github.com/project-serum/anchor/pull/792)).
* lang: Add `#[account(init_if_needed)]` keyword for allowing one to invoke the same instruction even if the account was created already ([#906](https://github.com/project-serum/anchor/pull/906)).
* lang: Add custom errors support for raw constraints ([#905](https://github.com/project-serum/anchor/pull/905)).
* lang, cli, spl: Update solana toolchain to v1.8.0 ([#886](https://github.com/project-serum/anchor/pull/886)).
* lang: Add custom errors support for `signer`, `mut`, `has_one`, `owner`, raw constraints and `address` ([#905](https://github.com/project-serum/anchor/pull/905), [#913](https://github.com/project-serum/anchor/pull/913)).

### Breaking

* lang: Accounts marked with the `#[account(signer)]` constraint now enforce signer when the `"cpi"` feature is enabled ([#849](https://github.com/project-serum/anchor/pull/849)).

## [0.17.0] - 2021-10-03

### Features

* cli: Add `localnet` command for starting a local `solana-test-validator` with the workspace deployed ([#820](https://github.com/project-serum/anchor/pull/820)).

### Breaking

* `CpiContext` accounts must now be used with the accounts struct generated in the `crate::cpi::accounts::*` module. These structs correspond to the accounts context for each instruction, except that each field is of type `AccountInfo` ([#824](https://github.com/project-serum/anchor/pull/824)).

## [0.16.2] - 2021-09-27

### Features

* lang: Add `--detach` flag to `anchor test` ([#770](https://github.com/project-serum/anchor/pull/770)).
* lang: Add `associated_token` keyword for initializing associated token accounts within `#[derive(Accounts)]` ([#790](https://github.com/project-serum/anchor/pull/790)).
* cli: Allow passing through cargo flags for build command ([#719](https://github.com/project-serum/anchor/pull/719)).
* cli: Allow passing through cargo flags for test, verify, and publish commands ([#804](https://github.com/project-serum/anchor/pull/804)).

### Fixes

* lang: Generated `AccountMeta`s for Rust clients now properly set the `isSigner` field ([#762](https://github.com/project-serum/anchor/pull/762)).

## [0.16.1] - 2021-09-17

### Fixes

* lang: `Signer` type now sets isSigner to true in the IDL ([#750](https://github.com/project-serum/anchor/pull/750)).

## [0.16.0] - 2021-09-16

### Features

* lang: `Program` type introduced for executable accounts ([#705](https://github.com/project-serum/anchor/pull/705)).
* lang: `Signer` type introduced for signing accounts where data is not used ([#705](https://github.com/project-serum/anchor/pull/705)).
* lang: `UncheckedAccount` type introduced as a preferred alias for `AccountInfo` ([#745](https://github.com/project-serum/anchor/pull/745)).

### Breaking Changes

* lang: `#[account(owner = <pubkey>)]` now requires a `Pubkey` instead of an account ([#691](https://github.com/project-serum/anchor/pull/691)).

## [0.15.0] - 2021-09-07

### Features

* lang: Add new `Account` type to replace `ProgramAccount` and `CpiAccount`, both of which are deprecated ([#686](https://github.com/project-serum/anchor/pull/686)).
* lang: `Box` can be used with `Account` types to reduce stack usage ([#686](https://github.com/project-serum/anchor/pull/686)).
* lang: Add `Owner` trait, which is automatically implemented by all `#[account]` structs ([#686](https://github.com/project-serum/anchor/pull/686)).
* lang: Check that ProgramAccount writable before mut borrow (`anchor-debug` only) ([#681](https://github.com/project-serum/anchor/pull/681)).

### Breaking Changes

* lang: All programs must now define their program id in source via `declare_id!` ([#686](https://github.com/project-serum/anchor/pull/686)).

## [0.14.0] - 2021-09-02

### Features

* lang: Ignore `Unnamed` structs instead of panic ([#605](https://github.com/project-serum/anchor/pull/605)).
* lang: Add constraints for initializing mint accounts as pdas, `#[account(init, seeds = [...], mint::decimals = <expr>, mint::authority = <expr>)]` ([#562](https://github.com/project-serum/anchor/pull/562)).
* lang: Add `AsRef<AccountInfo>` for `AccountInfo` wrappers ([#652](https://github.com/project-serum/anchor/pull/652)).
* lang: Optimize `trait Key` by removing `AccountInfo` cloning ([#652](https://github.com/project-serum/anchor/pull/652)).
* cli, client, lang: Update solana toolchain to v1.7.11 ([#653](https://github.com/project-serum/anchor/pull/653)).

### Breaking Changes

* lang: Change `#[account(init, seeds = [...], token = <expr>, authority = <expr>)]` to `#[account(init, token::mint = <expr> token::authority = <expr>)]` ([#562](https://github.com/project-serum/anchor/pull/562)).
* lang: `#[associated]` and `#[account(associated = <target>, with = <target>)]` are both removed ([#612](https://github.com/project-serum/anchor/pull/612)).
* cli: Removed `anchor launch` command ([#634](https://github.com/project-serum/anchor/pull/634)).
* lang: `#[account(init)]` now creates the account inside the same instruction to be consistent with initializing PDAs. To maintain the old behavior of `init`, replace it with `#[account(zero)]` ([#641](https://github.com/project-serum/anchor/pull/641)).
* lang: `bump` must be provided when using the `seeds` constraint. This has been added as an extra safety constraint to ensure that whenever a PDA is initialized via a constraint the bump used is the one created by `Pubkey::find_program_address` ([#641](https://github.com/project-serum/anchor/pull/641)).
* lang: `try_from_init` has been removed from `Loader`, `ProgramAccount`, and `CpiAccount`  and replaced with `try_from_unchecked` ([#641](https://github.com/project-serum/anchor/pull/641)).
* lang: Remove `AccountsInit` trait ([#641](https://github.com/project-serum/anchor/pull/641)).
* lang: `try_from` methods for `ProgramAccount`, `Loader`, and `ProgramState` now take in an additional `program_id: &Pubkey` parameter ([#660](https://github.com/project-serum/anchor/pull/660)).

## [0.13.2] - 2021-08-11

### Fixes

* cli: Fix `anchor init` command "Workspace not found" regression ([#598](https://github.com/project-serum/anchor/pull/598)).

## [0.13.1] - 2021-08-10

### Features

* cli: Programs embedded into genesis during tests will produce program logs ([#594](https://github.com/project-serum/anchor/pull/594)).

### Fixes

* cli: Allows Cargo.lock to exist in workspace subdirectories when publishing ([#593](https://github.com/project-serum/anchor/pull/593)).

## [0.13.0] - 2021-08-08

### Features

* cli: Adds a `[registry]` section in the Anchor toml ([#570](https://github.com/project-serum/anchor/pull/570)).
* cli: Adds the `anchor login <api-token>` command ([#570](https://github.com/project-serum/anchor/pull/570)).
* cli: Adds the `anchor publish <package>` command ([#570](https://github.com/project-serum/anchor/pull/570)).
* cli: Adds a root level `anchor_version` field to the Anchor.toml for specifying the anchor docker image to use for verifiable builds ([#570](https://github.com/project-serum/anchor/pull/570)).
* cli: Adds a root level `solana_version` field to the Anchor.toml for specifying the solana toolchain to use for verifiable builds ([#570](https://github.com/project-serum/anchor/pull/570)).
* lang: Dynamically fetch rent sysvar for when using `init` ([#587](https://github.com/project-serum/anchor/pull/587)).

### Breaking

* cli: `[clusters.<network>]` Anchor.toml section has been renamed to `[programs.<network>]` ([#570](https://github.com/project-serum/anchor/pull/570)).
* cli: `[workspace]` member and exclude arrays must now be filepaths relative to the workpsace root ([#570](https://github.com/project-serum/anchor/pull/570)).

## [0.12.0] - 2021-08-03

### Features

* cli: Add keys `members` / `exclude` in config `programs` section ([#546](https://github.com/project-serum/anchor/pull/546)).
* cli: Allow program address configuration for test command through `clusters.localnet` ([#554](https://github.com/project-serum/anchor/pull/554)).
* lang: IDLs are now parsed from the entire crate ([#517](https://github.com/project-serum/anchor/pull/517)).
* spl: Dex permissioned markets proxy ([#519](https://github.com/project-serum/anchor/pull/519), [#543](https://github.com/project-serum/anchor/pull/543)).

### Breaking Changes

* ts: Use `hex` by default for decoding Instruction ([#547](https://github.com/project-serum/anchor/pull/547)).
* lang: `CpiAccount::reload` mutates the existing struct instead of returning a new one ([#526](https://github.com/project-serum/anchor/pull/526)).
* cli: Anchor.toml now requires an explicit `[scripts]` test command ([#550](https://github.com/project-serum/anchor/pull/550)).

## [0.11.1] - 2021-07-09

### Features

* lang: Adds `require` macro for specifying assertions that return error codes on failure ([#483](https://github.com/project-serum/anchor/pull/483)).
* lang: Allow one to specify arbitrary programs as the owner when creating PDA ([#483](https://github.com/project-serum/anchor/pull/483)).
* lang: A new `bump` keyword is added to the accounts constraints, which is used to add an optional bump seed to the end of a `seeds` array. When used in conjunction with *both* `init` and `seeds`, then the program executes `find_program_address` to assert that the given bump is the canonical bump ([#483](https://github.com/project-serum/anchor/pull/483)).

### Fixes

* lang: Preserve all instruction data for fallback functions ([#483](https://github.com/project-serum/anchor/pull/483)).
* ts: Event listener not firing when creating associated accounts ([#356](https://github.com/project-serum/anchor/issues/356)).

## [0.11.0] - 2021-07-03

### Features

* lang: Add fallback functions ([#457](https://github.com/project-serum/anchor/pull/457)).
* lang: Add feature flag for using the old state account discriminator. This is a temporary flag for those with programs built prior to v0.7.0 but want to use the latest Anchor version. Expect this to be removed in a future version ([#446](https://github.com/project-serum/anchor/pull/446)).
* lang: Add generic support to Accounts ([#496](https://github.com/project-serum/anchor/pull/496)).

### Breaking Changes

* cli: Remove `.spec` suffix on TypeScript tests files ([#441](https://github.com/project-serum/anchor/pull/441)).
* lang: Remove `belongs_to` constraint ([#459](https://github.com/project-serum/anchor/pull/459)).

## [0.10.0] - 2021-06-27

### Features

* lang: Add `#[account(address = <expr>)]` constraint for asserting the address of an account ([#400](https://github.com/project-serum/anchor/pull/400)).
* lang: Add `#[account(init, token = <mint-target>, authority = <token-owner-target>...)]` constraint for initializing SPL token accounts as program derived addresses for the program. Can be used when initialized via `seeds` or `associated` ([#400](https://github.com/project-serum/anchor/pull/400)).
* lang: Add `associated_seeds!` macro for generating signer seeds for CPIs signed by an `#[account(associated = <target>)]` account ([#400](https://github.com/project-serum/anchor/pull/400)).
* cli: Add `[scripts]` section to the Anchor.toml for specifying workspace scripts that can be run via `anchor run <script>` ([#400](https://github.com/project-serum/anchor/pull/400)).
* cli: `[clusters.<network>]` table entries can now also use `{ address = <base58-str>, idl = <filepath-str> }` to specify workspace programs ([#400](https://github.com/project-serum/anchor/pull/400)).

### Breaking Changes

* cli: Remove `--yarn` flag in favor of using `npx` ([#432](https://github.com/project-serum/anchor/pull/432)).

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
