# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Note:** Version 0 of Semantic Versioning is handled differently from version 1 and above.
The minor version will be incremented upon a breaking change and the patch version will be incremented for features.

## [Unreleased]

## [0.25.0] - 2022-07-05

### Features

* lang: Add `realloc`, `realloc::payer`, and `realloc::zero` as a new constraint group for program accounts ([#1986](https://github.com/coral-xyz/anchor/pull/1986)).
* lang: Add `PartialEq` and `Eq` for `anchor_lang::Error` ([#1544](https://github.com/coral-xyz/anchor/pull/1544)).
* cli: Add `--skip-build` to `anchor publish` ([#1786](https://github.com/coral-xyz/anchor/pull/1841)).
* cli: Add `--program-keypair` to `anchor deploy` ([#1786](https://github.com/coral-xyz/anchor/pull/1786)).
* cli: Add compilation optimizations to cli template ([#1807](https://github.com/coral-xyz/anchor/pull/1807)).
* cli: `build` now adds docs to idl. This can be turned off with `--no-docs` ([#1561](https://github.com/coral-xyz/anchor/pull/1561)).
* cli: Add `b` and `t` aliases for `build` and `test` respectively ([#1823](https://github.com/coral-xyz/anchor/pull/1823)).
* spl: Add more derived traits to `TokenAccount` to `Mint` ([#1818](https://github.com/coral-xyz/anchor/pull/1818)).
* spl: Add `sync_native` token program CPI wrapper function ([#1833](https://github.com/coral-xyz/anchor/pull/1833)).
* cli: Allow passing arguments to an underlying script with `anchor run` ([#1914](https://github.com/coral-xyz/anchor/pull/1914)).
* ts: Implement a coder for system program ([#1920](https://github.com/coral-xyz/anchor/pull/1920)).
* ts: Add `program.coder.types` for encoding/decoding user-defined types ([#1931](https://github.com/coral-xyz/anchor/pull/1931)).
* client: Add send_with_spinner_and_config function to RequestBuilder ([#1926](https://github.com/coral-xyz/anchor/pull/1926)).
* ts: Implement a coder for SPL associated token program ([#1939](https://github.com/coral-xyz/anchor/pull/1939)).
* ts: verbose error for missing `ANCHOR_WALLET` variable when using `NodeWallet.local()` ([#1958](https://github.com/coral-xyz/anchor/pull/1958)).
* ts: Add `MethodsBuilder#accountsStrict` for strict typing on ix account input ([#2019](https://github.com/coral-xyz/anchor/pull/2019)).
* Update solana dependencies to 1.10.29  ([#2027](https://github.com/coral-xyz/anchor/pull/2027)).

### Fixes

* cli: Move `overflow-checks` into workspace `Cargo.toml` so that it will not be ignored by compiler ([#1806](https://github.com/coral-xyz/anchor/pull/1806)).
* lang: Fix missing account name information when deserialization fails when using `init` or `zero` ([#1800](https://github.com/coral-xyz/anchor/pull/1800)).
* ts: Expose the wallet's publickey on the Provider ([#1845](https://github.com/coral-xyz/anchor/pull/1845)).

### Breaking

* ts: Change `BROWSER` env variable to `ANCHOR_BROWSER` ([#1233](https://github.com/coral-xyz/anchor/pull/1233)).
* ts: Add transaction signature to `EventCallback` parameters ([#1851](https://github.com/coral-xyz/anchor/pull/1851)).
* ts: Change `EventParser#parseLogs` implementation to be a generator instead of callback function ([#2018](https://github.com/coral-xyz/anchor/pull/2018)).
* lang: Adds a new `&mut reallocs: BTreeSet<Pubkey>` argument to `Accounts::try_accounts` ([#1986](https://github.com/coral-xyz/anchor/pull/1986)).

## [0.24.2] - 2022-04-13

### Fixes

* lang: Fix `returns` being serialized as `null` instead of `undefined` in IDL ([#1782](https://github.com/coral-xyz/anchor/pull/1782)).

## [0.24.1] - 2022-04-12

### Fixes

* lang: Fix `anchor build` failing if `Test.toml` included a relative path that didn't exist yet because it's created by `anchor build` ([#1772](https://github.com/coral-xyz/anchor/pull/1772)).
* cli: Update js/ts template to use new `AnchorProvider` class ([#1770](https://github.com/coral-xyz/anchor/pull/1770)).

## [0.24.0] - 2022-04-12

### Features

* lang: Add support for multiple test suites with separate local validators ([#1681](https://github.com/coral-xyz/anchor/pull/1681)).
* lang: Add return values to CPI client ([#1598](https://github.com/coral-xyz/anchor/pull/1598)).
* ts: Add view functions ([#1695](https://github.com/coral-xyz/anchor/pull/1695)).
* avm: New `avm update` command to update the Anchor CLI to the latest version ([#1670](https://github.com/coral-xyz/anchor/pull/1670)).
* cli: Update js/ts templates to use new `program.methods` syntax ([#1732](https://github.com/coral-xyz/anchor/pull/1732)).
* cli: Workspaces created with `anchor init` now come with the `prettier` formatter and scripts included ([#1741](https://github.com/coral-xyz/anchor/pull/1741)).
* ts: Add `pubkeys` function to methods builder to get all instruction account addresses ([#1733](https://github.com/coral-xyz/anchor/pull/1733)).
* ts: Export `LangErrorCode` and `LangErrorMessage` from `error.ts` ([#1756](https://github.com/coral-xyz/anchor/pull/1756)).

### Fixes

* avm: `avm install` no longer downloads the version if already installed in the machine ([#1670](https://github.com/coral-xyz/anchor/pull/1670)).
* cli: make `anchor test` fail when used with `--skip-deploy` option and without `--skip-local-validator` option but there already is a running validator ([#1675](https://github.com/coral-xyz/anchor/pull/1675)).
* lang: Return proper error instead of panicking if account length is smaller than discriminator in functions of `(Account)Loader` ([#1678](https://github.com/coral-xyz/anchor/pull/1678)).
* cli: Add `@types/bn.js` to `devDependencies` in cli template ([#1712](https://github.com/coral-xyz/anchor/pull/1712)).
* ts: Event listener no longer crashes on Program Upgrade or any other unexpected log ([#1757](https://github.com/coral-xyz/anchor/pull/1757)).

### Breaking

* avm: `avm install` switches to the newly installed version after installation finishes ([#1670](https://github.com/coral-xyz/anchor/pull/1670)).
* spl: Re-export the `spl_token` crate ([#1665](https://github.com/coral-xyz/anchor/pull/1665)).
* lang, cli, spl: Update solana toolchain to v1.9.13 ([#1653](https://github.com/coral-xyz/anchor/pull/1653) and [#1751](https://github.com/coral-xyz/anchor/pull/1751)).
* lang: `Program` type now deserializes `programdata_address` only on demand ([#1723](https://github.com/coral-xyz/anchor/pull/1723)).
* ts: Make `Provider` an interface and adjust its signatures and add `AnchorProvider` implementor class ([#1707](https://github.com/coral-xyz/anchor/pull/1707)).
* spl: Change "to" to "from" in `token::burn` ([#1080](https://github.com/coral-xyz/anchor/pull/1080)).

## [0.23.0] - 2022-03-20

### Features

* cli: Add `anchor clean` command that's the same as `cargo clean` but preserves keypairs inside `target/deploy` ([#1470](https://github.com/coral-xyz/anchor/issues/1470)).
* cli: Running `anchor init` now initializes a new git repository for the workspace. This can be disabled with the `--no-git` flag ([#1605](https://github.com/coral-xyz/anchor/pull/1605)).
* cli: Add support for `anchor idl fetch` to work outside anchor workspace ([#1509](https://github.com/coral-xyz/anchor/pull/1509)).
* cli: [[test.validator.clone]] also clones the program data account of programs owned by the bpf upgradeable loader ([#1481](https://github.com/coral-xyz/anchor/issues/1481)).
* lang: Add new `AccountSysvarMismatch` error code and test cases for sysvars ([#1535](https://github.com/coral-xyz/anchor/pull/1535)).
* lang: Replace `std::io::Cursor` with a custom `Write` impl that uses the Solana mem syscalls ([#1589](https://github.com/coral-xyz/anchor/pull/1589)).
* lang: Add `require_neq`, `require_keys_neq`, `require_gt`, and `require_gte` comparison macros ([#1622](https://github.com/coral-xyz/anchor/pull/1622)).
* lang: Handle arrays with const as size in instruction data ([#1623](https://github.com/coral-xyz/anchor/issues/1623).
* spl: Add support for revoke instruction ([#1493](https://github.com/coral-xyz/anchor/pull/1493)).
* ts: Add provider parameter to `Spl.token` factory method ([#1597](https://github.com/coral-xyz/anchor/pull/1597)).

### Fixes

* ts: Fix the loss of strict typing using the `methods` namespace on builder functions ([#1539](https://github.com/coral-xyz/anchor/pull/1539)).
* spl: Update `spl/governance` to use new errors ([#1582](https://github.com/coral-xyz/anchor/pull/1582)).
* client: Fix `Cluster`'s `FromStr` implementation ([#1362](https://github.com/coral-xyz/anchor/pull/1362)).
* lang: Implement `Key` for `Pubkey` again, so `associated_token::*` constraints can use pubkey targets again ([#1601](https://github.com/coral-xyz/anchor/pull/1601)).
* lang: Adjust error code so `#[error_code]` works with just importing `anchor_lang::error_code` ([#1610](https://github.com/coral-xyz/anchor/pull/1610)).
* ts: Fix `spl-token` coder account parsing ([#1604](https://github.com/coral-xyz/anchor/pull/1604)).
* cli: Fix `npm install` fallback if `yarn` install doesn't work ([#1643](https://github.com/coral-xyz/anchor/pull/1643)).
* lang: Fix bug where `owner = <target>` would not compile because of missing type annotation ([#1648](https://github.com/coral-xyz/anchor/pull/1648)).
* ts: Adjust `send` and `simulate` functions in `provider.ts`, so they use the return value of `Wallet.signTransaction`([#1527](https://github.com/coral-xyz/anchor/pull/1527)).

### Breaking

* ts: Mark `transaction`, `instruction`, `simulate` and `rpc` program namespaces as deprecated in favor of `methods` ([#1539](https://github.com/coral-xyz/anchor/pull/1539)).
* ts: No longer allow manual setting of globally resolvable program public keys in `methods#accounts()`. ([#1548][https://github.com/coral-xyz/anchor/pull/1548])
* lang/ts: Events are now emitted using the `sol_log_data` syscall ([#1608](https://github.com/coral-xyz/anchor/pull/1608)).
* lang: Remove space calculation using `#[derive(Default)]` ([#1519](https://github.com/coral-xyz/anchor/pull/1519)).
* lang: Add support for logging expected and actual values and pubkeys. Add `require_eq` and `require_keys_eq` macros. Add default error code to `require` macro ([#1572](https://github.com/coral-xyz/anchor/pull/1572)).
* lang: Add `system_program` CPI wrapper functions. Make `system_program` module public instead of re-exporting `system_program::System`([#1629](https://github.com/coral-xyz/anchor/pull/1629)).
* cli: `avm use` no long prompts [y/n] if an install is needed first - it just tells the user to `avm install` ([#1565](https://github.com/coral-xyz/anchor/pull/1565))
* ts: Add `AnchorError` with program stack and also a program stack for non-`AnchorError` errors ([#1640](https://github.com/coral-xyz/anchor/pull/1640)). `AnchorError` is not returned for `processed` tx that have `skipPreflight` set to `true` (it falls back to `ProgramError` or the raw solana library error).

## [0.22.1] - 2022-02-28

### Fixes

* cli: Fix rust template ([#1488](https://github.com/coral-xyz/anchor/pull/1488)).
* lang: Handle array sizes with variable sizes in events and array size casting in IDL parsing ([#1485](https://github.com/coral-xyz/anchor/pull/1485))


## [0.22.0] - 2022-02-20

### Features

* lang: Add check that declared id == program id ([#1451](https://github.com/coral-xyz/anchor/pull/1451)).
* ts: Added float types support ([#1425](https://github.com/coral-xyz/anchor/pull/1425)).
* cli: Add `--skip-lint` option to disable check linting introduced in ([#1452](https://github.com/coral-xyz/anchor/pull/1452)) for rapid prototyping ([#1482](https://github.com/coral-xyz/anchor/pull/1482)).

### Fixes

* ts: Allow nullable types for `Option<T>` mapped types ([#1428](https://github.com/coral-xyz/anchor/pull/1428)).

### Breaking

* lang: Enforce that the payer for an init-ed account be marked `mut` ([#1271](https://github.com/coral-xyz/anchor/pull/1271)).
* lang: All error-related code is now in the error module ([#1426](https://github.com/coral-xyz/anchor/pull/1426)).
* lang: Require doc comments when using AccountInfo or UncheckedAccount types ([#1452](https://github.com/coral-xyz/anchor/pull/1452)).
* lang: add [`error!`](https://docs.rs/anchor-lang/latest/anchor_lang/prelude/macro.error.html) and [`err!`](https://docs.rs/anchor-lang/latest/anchor_lang/prelude/macro.err.html) macro and `Result` type ([#1462](https://github.com/coral-xyz/anchor/pull/1462)).
This change will break most programs. Do the following to upgrade:
     * change all `ProgramResult`'s to `Result<()>`
     * change `#[error]` to `#[error_code]`
     * change all `Err(MyError::SomeError.into())` to `Err(error!(MyError::SomeError))` and all `Err(ProgramError::SomeProgramError)` to `Err(ProgramError::SomeProgramError.into())` or `Err(Error::from(ProgramError::SomeProgramError).with_source(source!()))` to provide file and line source of the error (`with_source` is most useful with `ProgramError`s. `error!` already adds source information for custom and anchor internal errors).
     * change all `solana_program::program::invoke()` to `solana_program::program::invoke().map_err(Into::into)` and `solana_program::program::invoke_signed()` to `solana_program::program::invoke_signed().map_err(Into::into)`

## [0.21.0] - 2022-02-07

### Fixes

* ts: Fix the root type declaration of the `Wallet` / `NodeWallet` class ([#1363](https://github.com/coral-xyz/anchor/pull/1363)).
* ts: Improve type mapping of Account fields into Typescript with additional support for `Option<T>` and `Vec<String>` types ([#1393](https://github.com/coral-xyz/anchor/pull/1393)).

### Features

* lang: Add `seeds::program` constraint for specifying which program_id to use when deriving PDAs ([#1197](https://github.com/coral-xyz/anchor/pull/1197)).
* lang: `Context` now has a new `bumps: BTree<String, u8>` argument, mapping account name to bump seed "found" by the accounts context. This allows one to access bump seeds without having to pass them in from the client or recalculate them in the handler ([#1367](https://github.com/coral-xyz/anchor/pull/1367)).
* lang, ts: Automatically infer PDA addresses ([#1331](https://github.com/coral-xyz/anchor/pull/1331)).
* ts: Remove error logging in the event parser when log websocket encounters a program error ([#1313](https://github.com/coral-xyz/anchor/pull/1313)).
* ts: Add new `methods` namespace to the program client, introducing a more ergonomic builder API ([#1324](https://github.com/coral-xyz/anchor/pull/1324)).
* ts: Add registry utility for fetching the latest verified build ([#1371](https://github.com/coral-xyz/anchor/pull/1371)).
* cli: Expose the solana-test-validator --account flag in Anchor.toml via [[test.validator.account]] ([#1366](https://github.com/coral-xyz/anchor/pull/1366)).
* cli: Add avm, a tool for managing anchor-cli versions ([#1385](https://github.com/coral-xyz/anchor/pull/1385)).

### Breaking

* lang: Put `init_if_needed` behind a feature flag to decrease wrong usage ([#1258](https://github.com/coral-xyz/anchor/pull/1258)).
* lang: rename `loader_account` module to `account_loader` module ([#1279](https://github.com/coral-xyz/anchor/pull/1279))
* lang: The `Accounts` trait's `try_accounts` method now has an additional `bumps: &mut BTreeMap<String, u8>` argument, which accumulates bump seeds ([#1367](https://github.com/coral-xyz/anchor/pull/1367)).
* lang: Providing `bump = <target>` targets with `init` will now error. On `init` only, it is required to use `bump` without a target and access the seed inside function handlers via `ctx.bumps.get("<pda-account-name")`. For subsequent seeds constraints (without init), it is recommended to store the bump on your account and use it as a `bump = <target>` target to minimize compute units used ([#1380](https://github.com/coral-xyz/anchor/pull/1380)).
* ts: `Coder` is now an interface and the existing class has been renamed to `BorshCoder`. This change allows the generation of Anchor clients for non anchor programs  ([#1259](https://github.com/coral-xyz/anchor/pull/1259/files)).
* cli: [[test.clone]] key in Anchor.toml is renamed to [[test.validator.clone]] ([#1366](https://github.com/coral-xyz/anchor/pull/1366)).


## [0.20.1] - 2022-01-09

### Fixes

* lang: Improved error msgs when required programs are missing when using the `init` constraint([#1257](https://github.com/coral-xyz/anchor/pull/1257))

### Features

* lang: Allow repr overrides for zero copy accounts ([#1273](https://github.com/coral-xyz/anchor/pull/1273)).

## [0.20.0] - 2022-01-06

### Fixes

* lang: `init_if_needed` now checks rent exemption when init is not needed ([#1250](https://github.com/coral-xyz/anchor/pull/1250)).
* lang: Add missing owner check when `associated_token::authority` is used ([#1240](https://github.com/coral-xyz/anchor/pull/1240)).
* ts: Add type declarations for conditional `workspace` and `Wallet` exports ([#1137](https://github.com/coral-xyz/anchor/pull/1137)).
* ts: Change commitment message `recent` to `processed` and `max` to `finalized` ([#1128](https://github.com/coral-xyz/anchor/pull/1128))
* ts: fix `translateAddress` which currently leads to failing browser code. Now uses `PublicKey` constructor instead of prototype chain constructor name checking which doesn't work in the presence of code minifying/mangling([#1138](https://github.com/coral-xyz/anchor/pull/1138))
* lang: add missing check that verifies that account is ATA when using `init_if_needed` and init is not needed([#1221](https://github.com/coral-xyz/anchor/pull/1221))

### Features

* lang: Add `programdata_address: Option<Pubkey>` field to `Program` account. Will be populated if account is a program owned by the upgradable bpf loader ([#1125](https://github.com/coral-xyz/anchor/pull/1125))
* lang,ts,ci,cli,docs: update solana toolchain to version 1.8.5([#1133](https://github.com/coral-xyz/anchor/pull/1133)).
* lang: Account wrappers for non-Anchor programs no longer have to implement the `serialize` function because it has a default impl now. Similarly, they no longer have to implement `try_deserialize` which now delegates to `try_deserialize_unchecked` by default([#1156](https://github.com/coral-xyz/anchor/pull/1156)).
* lang: Add `set_inner` method to `Account<'a, T>` to enable easy updates ([#1177](https://github.com/coral-xyz/anchor/pull/1177)).
* lang: Handle arrays with const as length ([#968](https://github.com/coral-xyz/anchor/pull/968)).
* ts: Add optional commitment argument to `fetch` and `fetchMultiple` ([#1171](https://github.com/coral-xyz/anchor/pull/1171)).
* lang: Implement `AsRef<T>` for `Account<'a, T>`([#1173](https://github.com/coral-xyz/anchor/pull/1173))
* cli: Add `anchor expand` command which wraps around `cargo expand` ([#1160](https://github.com/coral-xyz/anchor/pull/1160))

### Breaking

* client: Client::new and Client::new_with_options now accept `Rc<dyn Signer>` instead of `Keypair` ([#975](https://github.com/coral-xyz/anchor/pull/975)).
* lang, ts: Change error enum name and message for 'wrong program ownership' account validation ([#1154](https://github.com/coral-xyz/anchor/pull/1154)).
* lang: Change from `#[repr(packed)]` to `#[repr(C)]` for zero copy accounts ([#1106](https://github.com/coral-xyz/anchor/pull/1106)).
* lang: Account types can now be found either in the `prelude` module or the `accounts` module but not longer directly under the root.
Deprecated account types are no longer imported by the prelude ([#1208](https://github.com/coral-xyz/anchor/pull/1208)).

## [0.19.0] - 2021-12-08

### Fixes

* lang: Add `deprecated` attribute to `ProgramAccount` ([#1014](https://github.com/coral-xyz/anchor/pull/1014)).
* cli: Add version number from programs `Cargo.toml` into extracted IDL ([#1061](https://github.com/coral-xyz/anchor/pull/1061)).
* lang: Add `deprecated` attribute to `Loader`([#1078](https://github.com/coral-xyz/anchor/pull/1078)).
* lang: the `init_if_needed` attribute now checks that given attributes (e.g. space, owner, token::authority etc.) are validated even when init is not needed ([#1096](https://github.com/coral-xyz/anchor/pull/1096)).

### Features

* lang: Add `ErrorCode::AccountNotInitialized` error to separate the situation when the account has the wrong owner from when it does not exist (#[1024](https://github.com/coral-xyz/anchor/pull/1024)).
* lang: Called instructions now log their name by default. This can be turned off with the `no-log-ix-name` flag ([#1057](https://github.com/coral-xyz/anchor/pull/1057)).
* lang: `ProgramData` and `UpgradableLoaderState` can now be passed into `Account` as generics. see [UpgradeableLoaderState](https://docs.rs/solana-program/latest/solana_program/bpf_loader_upgradeable/enum.UpgradeableLoaderState.html). `UpgradableLoaderState` can also be matched on to get `ProgramData`, but when `ProgramData` is used instead, anchor does the serialization and checking that it is actually program data for you  ([#1095](https://github.com/coral-xyz/anchor/pull/1095)).
* ts: Add better error msgs in the ts client if something wrong (i.e. not a pubkey or a string) is passed in as an account in an instruction accounts object ([#1098](https://github.com/coral-xyz/anchor/pull/1098)).
* ts: Add inputs `postInstructions` and `preInstructions` as a replacement for (the now deprecated) `instructions` ([#1007](https://github.com/coral-xyz/anchor/pull/1007)).
* ts: Add `getAccountInfo` helper method to account namespace/client ([#1084](https://github.com/coral-xyz/anchor/pull/1084)).

### Breaking

* lang, ts: Error codes have been mapped to new numbers to allow for more errors per namespace ([#1096](https://github.com/coral-xyz/anchor/pull/1096)).

## [0.18.2] - 2021-11-14

* cli: Replace global JavaScript dependency installs with local.

### Features

* lang: Add `SystemAccount<'info>` account type for generic wallet addresses or accounts owned by the system program ([#954](https://github.com/coral-xyz/anchor/pull/954))

### Fixes

* cli: fix dns in NODE_OPTIONS ([#928](https://github.com/coral-xyz/anchor/pull/928)).
* cli: output TypeScript IDL in `idl parse` subcommand ([#941](https://github.com/coral-xyz/anchor/pull/941)).
* cli: Add fields `os` and `cpu` to npm package `@project-serum/anchor-cli` ([#976](https://github.com/coral-xyz/anchor/pull/976)).
* cli: Allow specify output directory for TypeScript IDL ([#940](https://github.com/coral-xyz/anchor/pull/940)).

### Breaking

* spl: Move permissioned markets into dex repository ([#962](https://github.com/coral-xyz/anchor/pull/962)).

## [0.18.0] - 2021-10-24

### Features

* cli: Add support for configuration options for `solana-test-validator` in Anchor.toml ([#834](https://github.com/coral-xyz/anchor/pull/834)).
* cli: `target/types` directory now created on build to store a TypeScript types file for each program's IDL ([#795](https://github.com/coral-xyz/anchor/pull/795)).
* ts: `Program<T>` can now be typed with an IDL type ([#795](https://github.com/coral-xyz/anchor/pull/795)).
* lang: Add `mint::freeze_authority` keyword for mint initialization within `#[derive(Accounts)]` ([#835](https://github.com/coral-xyz/anchor/pull/835)).
* lang: Add `AccountLoader` type for `zero_copy` accounts with support for CPI ([#792](https://github.com/coral-xyz/anchor/pull/792)).
* lang: Add `#[account(init_if_needed)]` keyword for allowing one to invoke the same instruction even if the account was created already ([#906](https://github.com/coral-xyz/anchor/pull/906)).
* lang: Add custom errors support for raw constraints ([#905](https://github.com/coral-xyz/anchor/pull/905)).
* lang, cli, spl: Update solana toolchain to v1.8.0 ([#886](https://github.com/coral-xyz/anchor/pull/886)).
* lang: Add custom errors support for `signer`, `mut`, `has_one`, `owner`, raw constraints and `address` ([#905](https://github.com/coral-xyz/anchor/pull/905), [#913](https://github.com/coral-xyz/anchor/pull/913)).

### Breaking

* lang: Accounts marked with the `#[account(signer)]` constraint now enforce signer when the `"cpi"` feature is enabled ([#849](https://github.com/coral-xyz/anchor/pull/849)).

## [0.17.0] - 2021-10-03

### Features

* cli: Add `localnet` command for starting a local `solana-test-validator` with the workspace deployed ([#820](https://github.com/coral-xyz/anchor/pull/820)).

### Breaking

* `CpiContext` accounts must now be used with the accounts struct generated in the `crate::cpi::accounts::*` module. These structs correspond to the accounts context for each instruction, except that each field is of type `AccountInfo` ([#824](https://github.com/coral-xyz/anchor/pull/824)).

## [0.16.2] - 2021-09-27

### Features

* lang: Add `--detach` flag to `anchor test` ([#770](https://github.com/coral-xyz/anchor/pull/770)).
* lang: Add `associated_token` keyword for initializing associated token accounts within `#[derive(Accounts)]` ([#790](https://github.com/coral-xyz/anchor/pull/790)).
* cli: Allow passing through cargo flags for build command ([#719](https://github.com/coral-xyz/anchor/pull/719)).
* cli: Allow passing through cargo flags for test, verify, and publish commands ([#804](https://github.com/coral-xyz/anchor/pull/804)).

### Fixes

* lang: Generated `AccountMeta`s for Rust clients now properly set the `isSigner` field ([#762](https://github.com/coral-xyz/anchor/pull/762)).

## [0.16.1] - 2021-09-17

### Fixes

* lang: `Signer` type now sets isSigner to true in the IDL ([#750](https://github.com/coral-xyz/anchor/pull/750)).

## [0.16.0] - 2021-09-16

### Features

* lang: `Program` type introduced for executable accounts ([#705](https://github.com/coral-xyz/anchor/pull/705)).
* lang: `Signer` type introduced for signing accounts where data is not used ([#705](https://github.com/coral-xyz/anchor/pull/705)).
* lang: `UncheckedAccount` type introduced as a preferred alias for `AccountInfo` ([#745](https://github.com/coral-xyz/anchor/pull/745)).

### Breaking Changes

* lang: `#[account(owner = <pubkey>)]` now requires a `Pubkey` instead of an account ([#691](https://github.com/coral-xyz/anchor/pull/691)).

## [0.15.0] - 2021-09-07

### Features

* lang: Add new `Account` type to replace `ProgramAccount` and `CpiAccount`, both of which are deprecated ([#686](https://github.com/coral-xyz/anchor/pull/686)).
* lang: `Box` can be used with `Account` types to reduce stack usage ([#686](https://github.com/coral-xyz/anchor/pull/686)).
* lang: Add `Owner` trait, which is automatically implemented by all `#[account]` structs ([#686](https://github.com/coral-xyz/anchor/pull/686)).
* lang: Check that ProgramAccount writable before mut borrow (`anchor-debug` only) ([#681](https://github.com/coral-xyz/anchor/pull/681)).

### Breaking Changes

* lang: All programs must now define their program id in source via `declare_id!` ([#686](https://github.com/coral-xyz/anchor/pull/686)).

## [0.14.0] - 2021-09-02

### Features

* lang: Ignore `Unnamed` structs instead of panic ([#605](https://github.com/coral-xyz/anchor/pull/605)).
* lang: Add constraints for initializing mint accounts as pdas, `#[account(init, seeds = [...], mint::decimals = <expr>, mint::authority = <expr>)]` ([#562](https://github.com/coral-xyz/anchor/pull/562)).
* lang: Add `AsRef<AccountInfo>` for `AccountInfo` wrappers ([#652](https://github.com/coral-xyz/anchor/pull/652)).
* lang: Optimize `trait Key` by removing `AccountInfo` cloning ([#652](https://github.com/coral-xyz/anchor/pull/652)).
* cli, client, lang: Update solana toolchain to v1.7.11 ([#653](https://github.com/coral-xyz/anchor/pull/653)).

### Breaking Changes

* lang: Change `#[account(init, seeds = [...], token = <expr>, authority = <expr>)]` to `#[account(init, token::mint = <expr> token::authority = <expr>)]` ([#562](https://github.com/coral-xyz/anchor/pull/562)).
* lang: `#[associated]` and `#[account(associated = <target>, with = <target>)]` are both removed ([#612](https://github.com/coral-xyz/anchor/pull/612)).
* cli: Removed `anchor launch` command ([#634](https://github.com/coral-xyz/anchor/pull/634)).
* lang: `#[account(init)]` now creates the account inside the same instruction to be consistent with initializing PDAs. To maintain the old behavior of `init`, replace it with `#[account(zero)]` ([#641](https://github.com/coral-xyz/anchor/pull/641)).
* lang: `bump` must be provided when using the `seeds` constraint. This has been added as an extra safety constraint to ensure that whenever a PDA is initialized via a constraint the bump used is the one created by `Pubkey::find_program_address` ([#641](https://github.com/coral-xyz/anchor/pull/641)).
* lang: `try_from_init` has been removed from `Loader`, `ProgramAccount`, and `CpiAccount`  and replaced with `try_from_unchecked` ([#641](https://github.com/coral-xyz/anchor/pull/641)).
* lang: Remove `AccountsInit` trait ([#641](https://github.com/coral-xyz/anchor/pull/641)).
* lang: `try_from` methods for `ProgramAccount`, `Loader`, and `ProgramState` now take in an additional `program_id: &Pubkey` parameter ([#660](https://github.com/coral-xyz/anchor/pull/660)).

## [0.13.2] - 2021-08-11

### Fixes

* cli: Fix `anchor init` command "Workspace not found" regression ([#598](https://github.com/coral-xyz/anchor/pull/598)).

## [0.13.1] - 2021-08-10

### Features

* cli: Programs embedded into genesis during tests will produce program logs ([#594](https://github.com/coral-xyz/anchor/pull/594)).

### Fixes

* cli: Allows Cargo.lock to exist in workspace subdirectories when publishing ([#593](https://github.com/coral-xyz/anchor/pull/593)).

## [0.13.0] - 2021-08-08

### Features

* cli: Adds a `[registry]` section in the Anchor toml ([#570](https://github.com/coral-xyz/anchor/pull/570)).
* cli: Adds the `anchor login <api-token>` command ([#570](https://github.com/coral-xyz/anchor/pull/570)).
* cli: Adds the `anchor publish <package>` command ([#570](https://github.com/coral-xyz/anchor/pull/570)).
* cli: Adds a root level `anchor_version` field to the Anchor.toml for specifying the anchor docker image to use for verifiable builds ([#570](https://github.com/coral-xyz/anchor/pull/570)).
* cli: Adds a root level `solana_version` field to the Anchor.toml for specifying the solana toolchain to use for verifiable builds ([#570](https://github.com/coral-xyz/anchor/pull/570)).
* lang: Dynamically fetch rent sysvar for when using `init` ([#587](https://github.com/coral-xyz/anchor/pull/587)).

### Breaking

* cli: `[clusters.<network>]` Anchor.toml section has been renamed to `[programs.<network>]` ([#570](https://github.com/coral-xyz/anchor/pull/570)).
* cli: `[workspace]` member and exclude arrays must now be filepaths relative to the workpsace root ([#570](https://github.com/coral-xyz/anchor/pull/570)).

## [0.12.0] - 2021-08-03

### Features

* cli: Add keys `members` / `exclude` in config `programs` section ([#546](https://github.com/coral-xyz/anchor/pull/546)).
* cli: Allow program address configuration for test command through `clusters.localnet` ([#554](https://github.com/coral-xyz/anchor/pull/554)).
* lang: IDLs are now parsed from the entire crate ([#517](https://github.com/coral-xyz/anchor/pull/517)).
* spl: Dex permissioned markets proxy ([#519](https://github.com/coral-xyz/anchor/pull/519), [#543](https://github.com/coral-xyz/anchor/pull/543)).

### Breaking Changes

* ts: Use `hex` by default for decoding Instruction ([#547](https://github.com/coral-xyz/anchor/pull/547)).
* lang: `CpiAccount::reload` mutates the existing struct instead of returning a new one ([#526](https://github.com/coral-xyz/anchor/pull/526)).
* cli: Anchor.toml now requires an explicit `[scripts]` test command ([#550](https://github.com/coral-xyz/anchor/pull/550)).

## [0.11.1] - 2021-07-09

### Features

* lang: Adds `require` macro for specifying assertions that return error codes on failure ([#483](https://github.com/coral-xyz/anchor/pull/483)).
* lang: Allow one to specify arbitrary programs as the owner when creating PDA ([#483](https://github.com/coral-xyz/anchor/pull/483)).
* lang: A new `bump` keyword is added to the accounts constraints, which is used to add an optional bump seed to the end of a `seeds` array. When used in conjunction with *both* `init` and `seeds`, then the program executes `find_program_address` to assert that the given bump is the canonical bump ([#483](https://github.com/coral-xyz/anchor/pull/483)).

### Fixes

* lang: Preserve all instruction data for fallback functions ([#483](https://github.com/coral-xyz/anchor/pull/483)).
* ts: Event listener not firing when creating associated accounts ([#356](https://github.com/coral-xyz/anchor/issues/356)).

## [0.11.0] - 2021-07-03

### Features

* lang: Add fallback functions ([#457](https://github.com/coral-xyz/anchor/pull/457)).
* lang: Add feature flag for using the old state account discriminator. This is a temporary flag for those with programs built prior to v0.7.0 but want to use the latest Anchor version. Expect this to be removed in a future version ([#446](https://github.com/coral-xyz/anchor/pull/446)).
* lang: Add generic support to Accounts ([#496](https://github.com/coral-xyz/anchor/pull/496)).

### Breaking Changes

* cli: Remove `.spec` suffix on TypeScript tests files ([#441](https://github.com/coral-xyz/anchor/pull/441)).
* lang: Remove `belongs_to` constraint ([#459](https://github.com/coral-xyz/anchor/pull/459)).

## [0.10.0] - 2021-06-27

### Features

* lang: Add `#[account(address = <expr>)]` constraint for asserting the address of an account ([#400](https://github.com/coral-xyz/anchor/pull/400)).
* lang: Add `#[account(init, token = <mint-target>, authority = <token-owner-target>...)]` constraint for initializing SPL token accounts as program derived addresses for the program. Can be used when initialized via `seeds` or `associated` ([#400](https://github.com/coral-xyz/anchor/pull/400)).
* lang: Add `associated_seeds!` macro for generating signer seeds for CPIs signed by an `#[account(associated = <target>)]` account ([#400](https://github.com/coral-xyz/anchor/pull/400)).
* cli: Add `[scripts]` section to the Anchor.toml for specifying workspace scripts that can be run via `anchor run <script>` ([#400](https://github.com/coral-xyz/anchor/pull/400)).
* cli: `[clusters.<network>]` table entries can now also use `{ address = <base58-str>, idl = <filepath-str> }` to specify workspace programs ([#400](https://github.com/coral-xyz/anchor/pull/400)).

### Breaking Changes

* cli: Remove `--yarn` flag in favor of using `npx` ([#432](https://github.com/coral-xyz/anchor/pull/432)).

## [0.9.0] - 2021-06-15

### Features

* lang: Instruction data is now available to accounts constraints ([#386](https://github.com/coral-xyz/anchor/pull/386)).
* lang: Initialize program derived addresses with accounts constraints ([#386](https://github.com/coral-xyz/anchor/pull/386)).

### Breaking Changes

* lang: Event field names in IDLs are now mixed case. ([#379](https://github.com/coral-xyz/anchor/pull/379)).
* lang: Accounts trait now accepts an additional `&[u8]` parameter ([#386](https://github.com/coral-xyz/anchor/pull/386)).

## [0.8.0] - 2021-06-10

### Features

* cli: Add `--program-name` option for build command to build a single program at a time ([#362](https://github.com/coral-xyz/anchor/pull/362)).
* cli, client: Parse custom cluster urls from str ([#369](https://github.com/coral-xyz/anchor/pull/369)).
* cli, client, lang: Update solana toolchain to v1.7.1 ([#368](https://github.com/coral-xyz/anchor/pull/369)).
* ts: Instruction decoding and formatting ([#372](https://github.com/coral-xyz/anchor/pull/372)).
* lang: Add `#[account(close = <destination>)]` constraint for closing accounts and sending the rent exemption lamports to a specified destination account ([#371](https://github.com/coral-xyz/anchor/pull/371)).

### Fixes

* lang: Allows one to use `remaining_accounts` with `CpiContext` by implementing the `ToAccountMetas` trait on `CpiContext` ([#351](https://github.com/coral-xyz/anchor/pull/351/files)).

### Breaking

* lang, ts: Framework defined error codes are introduced, reserving error codes 0-300 for Anchor, and 300 and up for user defined error codes ([#354](https://github.com/coral-xyz/anchor/pull/354)).

## [0.7.0] - 2021-05-31

### Features

* cli: Add global options for override Anchor.toml values ([#313](https://github.com/coral-xyz/anchor/pull/313)).
* spl: Add `SetAuthority` instruction ([#307](https://github.com/coral-xyz/anchor/pull/307/files)).
* spl: Add init and close open orders instructions ([#245](https://github.com/coral-xyz/anchor/pull/245)).
* lang: `constraint = <expression>` added as a replacement for (the now deprecated) string literal constraints ([#341](https://github.com/coral-xyz/anchor/pull/341)).
* lang: Span information is now preserved, providing informative compiler error messages ([#341](https://github.com/coral-xyz/anchor/pull/341)).
* ts: Address metadata is now optional for `anchor.workspace` clients ([#310](https://github.com/coral-xyz/anchor/pull/310)).

### Breaking Changes

* ts: Retrieving deserialized accounts from the `<program>.account.<my-account>` and `<program>.state` namespaces now require explicitly invoking the `fetch` API. For example, `program.account.myAccount(<adddress>)` and `program.state()` is now `program.account.myAccount.fetch(<address>)` and `program.state.fetch()` ([#322](https://github.com/coral-xyz/anchor/pull/322)).
* lang: `#[account(associated)]` now requires `init` to be provided to create an associated account. If not provided, then the address will be assumed to exist, and a constraint will be added to ensure the correctness of the address ([#318](https://github.com/coral-xyz/anchor/pull/318)).
* lang, ts: Change account discriminator pre-image of the `#[state]` account discriminator to be namespaced by "state:" ([#320](https://github.com/coral-xyz/anchor/pull/320)).
* lang, ts: Change domain delimiters for the pre-image of the instruciton sighash to be a single colon `:` to be consistent with accounts ([#321](https://github.com/coral-xyz/anchor/pull/321)).
* lang: Associated constraints no longer automatically implement `mut` ([#341](https://github.com/coral-xyz/anchor/pull/341)).
* lang: Associated `space` constraints must now be literal integers instead of literal strings ([#341](https://github.com/coral-xyz/anchor/pull/341)).

## [0.6.0] - 2021-05-23

### Features

* ts: Add `program.simulate` namespace ([#266](https://github.com/coral-xyz/anchor/pull/266)).
* ts: Introduce `Address` type, allowing one to use Base 58 encoded strings in public APIs ([#304](https://github.com/coral-xyz/anchor/pull/304)).
* ts: Replace deprecated `web3.Account` with `web3.Signer` in public APIs ([#296](https://github.com/coral-xyz/anchor/pull/296)).
* ts: Generated `anchor.workspace` clients can now be customized per network with `[cluster.<slug>]` in the Anchor.toml ([#308](https://github.com/coral-xyz/anchor/pull/308)).
* cli: Add yarn flag to test command ([#267](https://github.com/coral-xyz/anchor/pull/267)).
* cli: Add `--skip-build` flag to test command ([301](https://github.com/coral-xyz/anchor/pull/301)).
* cli: Add `anchor shell` command to spawn a node shell populated with an Anchor.toml based environment ([#303](https://github.com/coral-xyz/anchor/pull/303)).

### Breaking Changes

* cli: The Anchor.toml's `wallet` and `cluster` settings must now be under the `[provider]` table ([#305](https://github.com/coral-xyz/anchor/pull/305)).
* ts: Event coder `decode` API changed to decode strings directly instead of buffers ([#292](https://github.com/coral-xyz/anchor/pull/292)).
* ts: Event coder `encode` API removed ([#292](https://github.com/coral-xyz/anchor/pull/292)).

## [0.5.0] - 2021-05-07

### Features

* client: Adds support for state instructions ([#248](https://github.com/coral-xyz/anchor/pull/248)).
* lang: Add `anchor-debug` feature flag for logging ([#253](https://github.com/coral-xyz/anchor/pull/253)).
* ts: Add support for u16 ([#255](https://github.com/coral-xyz/anchor/pull/255)).

### Breaking Changes

* client: Renames `RequestBuilder::new` to `RequestBuilder::from` ([#248](https://github.com/coral-xyz/anchor/pull/248)).
* lang: Renames the generated `instruction::state::Ctor` struct to `instruction::state::New` ([#248](https://github.com/coral-xyz/anchor/pull/248)).

## [0.4.5] - 2021-04-29

* spl: Add serum DEX CPI client ([#224](https://github.com/coral-xyz/anchor/pull/224)).

## [0.4.4] - 2021-04-18

### Features

* lang: Allows one to specify multiple `with` targets when creating associated acconts ([#197](https://github.com/coral-xyz/anchor/pull/197)).
* lang, ts: Add array support ([#202](https://github.com/coral-xyz/anchor/pull/202)).
* lang: Zero copy deserialization for accounts ([#202](https://github.com/coral-xyz/anchor/pull/202), [#206](https://github.com/coral-xyz/anchor/pull/206)).
* lang, spl, cli, client: Upgrade solana toolchain to 1.6.6 ([#210](https://github.com/coral-xyz/anchor/pull/210)).

## [0.4.3] - 2021-04-13

### Features

* lang: CPI clients for program state instructions ([#43](https://github.com/coral-xyz/anchor/pull/43)).
* lang: Add `#[account(owner = <program>)]` constraint ([#178](https://github.com/coral-xyz/anchor/pull/178)).
* lang, cli, ts: Add `#[account(associated = <target>)]` and `#[associated]` attributes for creating associated program accounts within programs. The TypeScript package can fetch these accounts with a new `<program>.account.<account-name>.associated` (and `associatedAddress`) method ([#186](https://github.com/coral-xyz/anchor/pull/186)).

### Fixes

* lang: Unused `#[account]`s are now parsed into the IDL correctly ([#177](https://github.com/coral-xyz/anchor/pull/177)).

## [0.4.2] - 2021-04-10

### Features

* cli: Fund Anchor.toml configured wallet when testing ([#164](https://github.com/coral-xyz/anchor/pull/164)).
* spl: Add initialize_account instruction for spl tokens ([#166](https://github.com/coral-xyz/anchor/pull/166)).

## [0.4.1] - 2021-04-06

* cli: Version verifiable docker builder ([#145](https://github.com/coral-xyz/anchor/pull/145)).

## [0.4.0] - 2021-04-04

### Features

* cli: Specify test files to run ([#118](https://github.com/coral-xyz/anchor/pull/118)).
* lang: Allow overriding the `#[state]` account's size ([#121](https://github.com/coral-xyz/anchor/pull/121)).
* lang, client, ts: Add event emission and subscriptions ([#89](https://github.com/coral-xyz/anchor/pull/89)).
* lang/account: Allow namespacing account discriminators ([#128](https://github.com/coral-xyz/anchor/pull/128)).
* cli: TypeScript migrations ([#132](https://github.com/coral-xyz/anchor/pull/132)).
* lang: Add `#[account(executable)]` attribute ([#140](https://github.com/coral-xyz/anchor/pull/140)).

### Breaking Changes

* client: Replace url str with `Cluster` struct when constructing clients ([#89](https://github.com/coral-xyz/anchor/pull/89)).
* lang: Changes the account discriminator of `IdlAccount` to be namespaced by `"internal"` ([#128](https://github.com/coral-xyz/anchor/pull/128)).
* lang, spl, cli: Upgrade solana toolchain to 1.6.3, a major version upgrade even though only the minor version is incremented. This allows for the removal of `-#![feature(proc_macro_hygiene)]`. ([#139](https://github.com/coral-xyz/anchor/pull/139)).

## [0.3.0] - 2021-03-12

### Features

* ts: Allow preloading instructions for state rpc transactions ([cf9c84](https://github.com/coral-xyz/anchor/commit/cf9c847e4144989b5bc1936149d171e90204777b)).
* ts: Export sighash coder function ([734c75](https://github.com/coral-xyz/anchor/commit/734c751882f43beec7ea3f0f4d988b502e3f24e4)).
* cli: Specify programs to embed into local validator genesis via Anchor.toml while testing ([b3803a](https://github.com/coral-xyz/anchor/commit/b3803aec03fbbae1a794c9aa6a789e6cb58fda99)).
* cli: Allow skipping the creation of a local validator when testing against localnet ([#93](https://github.com/coral-xyz/anchor/pull/93)).
* cli: Adds support for tests with Typescript ([#94](https://github.com/coral-xyz/anchor/pull/94)).
* cli: Deterministic and verifiable builds ([#100](https://github.com/coral-xyz/anchor/pull/100)).
* cli, lang: Add write buffers for IDL upgrades ([#107](https://github.com/coral-xyz/anchor/pull/107)).

## Breaking Changes

* lang: Removes `IdlInstruction::Clear` ([#107](https://github.com/coral-xyz/anchor/pull/107)).

### Fixes

* cli: Propagates mocha test exit status on error ([79b791](https://github.com/coral-xyz/anchor/commit/79b791ffa85ffae5b6163fa853562aa568650f21)).

## [0.2.1] - 2021-02-11

### Features

* cli: Embed workspace programs into local validator genesis when testing ([733ec3](https://github.com/coral-xyz/anchor/commit/733ec300b0308e7d007873b0975585d836333fd4)).
* cli: Stream program logs to `.anchor/program-logs` directory when testing ([ce5ca7](https://github.com/coral-xyz/anchor/commit/ce5ca7ddab6e0fd579deddcd02094b3f798bbe6a)).
* spl: Add shared memory api [(d92cb1)](https://github.com/coral-xyz/anchor/commit/d92cb1516b78696d1257e41d0c5ac6821716300e).
* lang/attribute/access-control: Allow specifying multiple modifier functions ([845df6](https://github.com/coral-xyz/anchor/commit/845df6d1960bb544fa0f2e3331ec79cc804edeb6)).
* lang/syn: Allow state structs that don't have a ctor or impl block (just trait implementations) ([a78000](https://github.com/coral-xyz/anchor/commit/a7800026833d64579e5b19c90d724ecc20d2a455)).
* ts: Add instruction method to state namespace ([627c27](https://github.com/coral-xyz/anchor/commit/627c275e9cdf3dafafcf44473ba8146cc7979d44)).
* lang/syn, ts: Add support for u128 and i128 ([#83](https://github.com/coral-xyz/anchor/pull/83)).

## [0.2.0] - 2021-02-08

### Features

* lang: Adds the ability to create and use CPI program interfaces ([#66](https://github.com/coral-xyz/anchor/pull/66/files?file-filters%5B%5D=)).

### Breaking Changes

* lang, client, ts: Migrate from rust enum based method dispatch to a variant of sighash ([#64](https://github.com/coral-xyz/anchor/pull/64)).

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
