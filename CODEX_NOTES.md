# Codex Notes: Anchor v2 Docs Branch

Purpose: durable working memory for future Codex turns after context compaction.
Read this file first before re-learning Anchor v2 from scratch.

## Branch State

- Repo: `jktrn-osec/anchor`, local path `C:\Users\Jason\Desktop\Main\Code\Work\anchor`.
- Working branch: `enscribe/v2-docs`.
- PR target context: `solana-foundation/anchor` branch `anchor-next`.
- Last sync performed by Codex: merged `upstream/anchor-next` into `enscribe/v2-docs`.
- Merge commit: `307352cc Merge remote-tracking branch 'upstream/anchor-next' into enscribe/v2-docs`.
- Upstream synced through: `3fda2486 v2: Improve support for more seed expressions in IDL output (#4489)`.
- Push completed to `origin enscribe/v2-docs`.
- After sync: worktree was clean, `upstream/anchor-next` was an ancestor of `HEAD`, and `git diff --check HEAD^1..HEAD` passed.
- Full test suite was not run after the sync.

## What Anchor Is

Anchor is a framework for Solana programs:

- Rust eDSL and macros for program handlers, accounts, constraints, events, errors, and IDL generation.
- IDL spec for describing program instructions, accounts, types, events, and errors.
- TypeScript/Rust clients generated from or driven by the IDL.
- CLI/workspace tooling for init, build, test, deploy, IDL, profiling, debugger, and workspace workflows.

## Anchor v1 Baseline

- Stable published path lives under `lang/`, `spl/`, `ts/`, `cli/`, etc.
- Uses `solana_program`/Solana SDK account model and many `<'info>` lifetimes.
- User account wrapper is `Account<'info, T>`.
- v1 `Account` borsh-deserializes account data into an owned `T` on load and serializes on exit.
- `AccountLoader<'info, T>` is the usual v1 zero-copy route.
- `Context<T>` is passed by value in handlers.
- CPI accounts are usually built from `AccountInfo` clones through `ToAccountInfos`/`ToAccountMetas`.
- Duplicate mutable account protection in v1 is more macro-generated and set-based.

## Anchor v2 Summary

- New runtime crate: `lang-v2/` (`anchor-lang-v2`).
- New SPL crate: `spl-v2/` (`anchor-spl-v2`).
- Test harness crate: `v2-testing/` (`anchor-v2-testing`).
- Assembly helper crate: `asm-v2/` (`anchor-asm-v2`).
- v2 is alpha, not audited, and not on crates.io. Depend via git on `anchor-next`.
- v2 keeps the Anchor product shape but rewrites the runtime around:
  - `#![no_std]`
  - pinocchio
  - trait-based account wrappers and constraints
  - zero-copy-by-default account data
  - wincode instruction/event encoding with borsh-wire-compatible config
  - first-class profiling, coverage, and debugger tooling

## v1 to v2 Migration Map

- `use anchor_lang::prelude::*` -> `use anchor_lang_v2::prelude::*`
- `solana_program` -> `pinocchio` plus re-exported compatibility pieces where needed.
- `std` required -> `#![no_std]` compatible. `alloc` is a default feature.
- `Context<T>` handler arg -> `&mut Context<T>`.
- `<'info>` lifetimes on wrappers -> removed.
- `Pubkey` -> `Address`.
- `Account<'info, T>` for fixed-size data -> `Account<T>` with `T: Pod`.
- `Account<'info, T>` for `Vec`, `String`, payload enums -> `BorshAccount<T>`.
- v1 `AccountLoader<'info, T>` zero-copy -> v2 `Account<T>` zero-copy by default.
- v2 has a type named `AccountLoader`, but it is an internal sequential cursor walker, not the v1 user wrapper.
- `ctx.accounts.foo.key()` -> often `ctx.accounts.foo.address()` or `ctx.accounts.foo.account().address()`.
- `ctx.bumps.get("foo")` -> typed `ctx.bumps.foo`.
- `has_one = field` still parses but is deprecated. Prefer `address = parent.field` on the sibling account.
- Larger v1 ports can enable `features = ["compat"]` for `debug!`, a v1-shaped formatting logger. It is off by default because it allocates.

## v2 Account Model

- All wrappers implement `AnchorAccount` from `lang-v2/src/traits.rs`.
- `Account<T>` is a type alias for `Slab<T, HeaderOnly>`.
- `Account<T>` layout for native Anchor accounts: `[8-byte discriminator][repr(C) T]`.
- `#[account]` in v2 enforces Pod-compatible fixed-size layout:
  - `T: bytemuck::Pod`
  - `#[repr(C)]`
  - discriminator impl
  - owner impl
  - no-padding assertion
- Use `BorshAccount<T>` for variable-length fields. It holds a pinocchio borrow guard, deserializes on load, and serializes on `exit`.
- `BorshAccount<T>` has `release_borrow()` and `reacquire_borrow_mut()` for CPI/realloc cases where another path mutates the same account.
- `Slab<H, Item>` is the dynamic-tail primitive: typed header plus length-prefixed tail.
- `Nested<T>` composes `#[derive(Accounts)]` structs inline without consuming an extra account slot.
- `Option<Account<T>>` uses program ID as the absent sentinel; bumps become `Option<u8>`.
- `Signer`, `Program<T>`, `SystemAccount`, `UncheckedAccount`, and `Sysvar<T>` are v1-shaped wrappers without `<'info>`.
- `UncheckedAccount` remains an escape hatch and should be paired with explicit `address`, `owner`, or custom validation.

## v2 Constraint and Macro Model

- `#[program]` emits dispatcher, instruction structs, accounts re-exports, IDL test hook, and entrypoint unless `no-entrypoint`.
- Handlers use auto-derived 8-byte discriminators by default: `sha256("global:" + name)[..8]`.
- Handlers can use compact `#[discrim = N]` u8 discriminators. If one handler uses it, all handlers in that program module must use unique u8 values.
- `#[derive(Accounts)]` emits:
  - `TryAccounts`
  - typed bumps
  - `MUT_MASK`
  - `Resolved` client accounts struct
  - `to_account_metas()`
  - PDA finder helpers
  - IDL account metadata under `idl-build`
- Account validation walks account views once, then uses the generated `TryAccounts` impl.
- Duplicate mutable account detection uses a 256-bit mask (`[u64; 4]`) plus runtime duplicate bitvec. This is much cheaper than per-pair checks.
- `unsafe(dup)` is required to opt out of duplicate-account protection.
- Namespaced constraints are routed through `AccountConstraint<A>` with lifecycle hooks:
  - `init`
  - `check`
  - `update`
  - `exit`
- This lets third-party crates add constraints without forking the derive macro.
- `anchor-spl-v2` token/mint/associated-token constraints are built on this extension pattern.

## CPI Model

- `CpiContext<'a, T>` bundles typed CPI accounts, target program address, signer seeds, and remaining accounts.
- Accounts are passed as `CpiHandle<'a>` from `account.cpi_handle()` or `account.cpi_handle_mut()`.
- `CpiHandle` intentionally does not deref to `AccountView`, preventing accidental use with checked pinocchio invoke builders.
- `CpiContext::invoke(data)` calls pinocchio `invoke_signed_unchecked`.
- Safety argument: Rust borrows from `CpiHandle` prevent typed mutable aliasing while CPI is in flight; stale raw borrow cases are still blocked by pinocchio borrow state where relevant.
- v2 `Resolved` structs reduce caller boilerplate. They include only caller-supplied fields; PDAs and well-known programs auto-fill/auto-derive in `to_account_metas()`.
- Recent upstream sync included `528b790d v2: CPI account generation (#4486)`.

## Performance Sources

The speed/size win is cumulative:

- no_std + pinocchio instead of heavier Solana SDK path
- zero-copy default `Account<T>`
- `PodU*`, `PodI*`, `PodBool`, and `PodVec<T, MAX>` for layout-safe fixed-size data
- macro-time PDA bump precomputation for literal seeds
- hash-only PDA verification when the account type proves non-empty program-owned data
- wincode for instruction args/events with borsh-compatible wire config
- `#[event(bytemuck)]` for fixed-size events
- typed CPI handles enabling unchecked CPI without runtime borrow checks
- `const-rent` optional feature skips `Rent::get()` for create-account rent
- `guardrails` and `account-resize` can be disabled for smaller production binaries when appropriate
- `remaining_accounts()` is lazy-cached
- mask-based duplicate-account detection
- smaller core derive and more logic behind traits

Checked-in `bench/results.json` examples after earlier read:

- `hello_world_v2`: about 6.4 KB vs v1 about 124.6 KB; init about 1381 CU vs 5855.
- `vault_v2`: about 5.4 KB vs v1 about 107 KB; withdraw about 389 CU vs 2478.
- `nested_v2`: about 12.4 KB vs v1 about 157 KB; increment about 474 CU vs 4751.
- `prop_amm_v2`: about 8.6 KB vs v1 about 140 KB; update about 26 CU vs 1310, helped by asm fast path.
- `multisig_v2`: about 31 KB vs v1 about 170 KB; create/deposit/execute/set_label all materially cheaper.

## Feature Flags

`anchor-lang-v2` default features:

- `alloc`
- `guardrails`
- `account-resize`

Opt-in features:

- `const-rent`
- `compat`
- `idl-build`
- `testing`

Notes:

- `guardrails` catches runtime misuse like wrong program ID or mutable access to read-only accounts; disabling saves size/CU.
- `account-resize` enables realloc hooks and should not be disabled if the program calls realloc helpers.
- `const-rent` bakes rent formula constants and risks drift if Solana changes rent math.
- `idl-build` should not ship in production binaries.
- `testing` exposes host-side mock account scaffolding.

## IDL and TS Notes

- v2 IDL emission is trait-driven through `IdlAccountType` and `IdlType`.
- IDL includes docs, PDA seed metadata, optional accounts, relations, events, constants, error codes, serialization tags, repr, generics, and transitive type deps.
- Recent upstream sync included `3fda2486 v2: Improve support for more seed expressions in IDL output (#4489)`.
- `BorshInstructionCoder` handles variable-length discriminators by reading `layout.discriminator.length`.
- Event parser was fixed to push the invoked program ID for CPI/self-CPI rather than literal `"cpi"`.
- TS scaffold is still pinned to `@anchor-lang/core` `^1.0.0` until v2 package publishing lands.

## SPL v2

- `spl-v2/` mirrors v1 `anchor-spl` split.
- Provides zero-copy `Mint` and `TokenAccount` layouts.
- Provides `InterfaceAccount<T>` for accepting both Token and Token-2022 ownership.
- Provides TLV parsing helpers for Token-2022 extensions.
- Some SPL constraints are still incomplete during alpha, especially less-common token metadata surfaces.

## Testing, Debugging, Coverage

- `anchor-v2-testing::svm()` is a drop-in LiteSVM constructor.
- With `profile` feature it records SBF register traces under `target/anchor-v2-profile/<test>/`.
- `anchor test --profile` enables the trace path and renders flamegraphs.
- `anchor debugger` consumes the same traces and opens an instruction-level TUI.
- `anchor debugger --gdb` uses the sbpf gdb-stub path.
- `anchor coverage` maps register traces to source lines and emits LCOV.
- Debugger/profiler work best with DWARF preserved; CLI sets relevant `CARGO_PROFILE_RELEASE_DEBUG` and rustc wrapper bits.

## Important Files To Re-read

- `lang-v2/README.md`
- `docs-v2/src/content/docs/v2/index.mdx`
- `docs-v2/src/content/docs/v2/migration.mdx`
- `docs-v2/src/content/docs/v2/account-types.mdx`
- `docs-v2/src/content/docs/v2/optimizations.mdx`
- `docs-v2/src/content/docs/v2/cpi.mdx`
- `docs-v2/src/content/docs/v2/extensibility.mdx`
- `docs-v2/src/content/docs/v2/testing-and-debugging.mdx`
- `lang-v2/src/lib.rs`
- `lang-v2/src/traits.rs`
- `lang-v2/src/dispatch.rs`
- `lang-v2/src/context.rs`
- `lang-v2/src/context_cpi.rs`
- `lang-v2/src/accounts/mod.rs`
- `lang-v2/src/accounts/slab.rs`
- `lang-v2/src/accounts/borsh_account.rs`
- `lang-v2/src/pod.rs`
- `lang-v2/derive/src/lib.rs`
- `lang-v2/derive/src/parse.rs`
- `spl-v2/src/lib.rs`
- `spl-v2/src/token.rs`
- `spl-v2/src/mint.rs`
- `spl-v2/src/token_interface.rs`
- `spl-v2/src/extensions.rs`
- `bench/results.json`
- `bench/programs/*/anchor-v1/src`
- `bench/programs/*/anchor-v2/src`
- `cli/src/debugger/*`
- `v2-testing/src/lib.rs`
- `v2-testing/src/profile.rs`

## Avoid Repeating This Mistake

- Do not dump whole large files unless absolutely needed.
- Use targeted `Select-String`, `git show --stat`, `git diff --name-only`, and small file slices.
- `rg` was access-denied in this Windows environment during the first pass; use `git ls-files` and PowerShell `Select-String` if that persists.
- Keep future summaries anchored to this note instead of re-reading all docs and source.

## Refinements From Targeted Pass: 2026-04-30

Latest upstream v2 commits worth remembering:

- `528b790d v2: CPI account generation (#4486)` added generated on-chain CPI account structs and wrapper functions.
- `0f42cdf9 feat(v2): Add more pinocchio sysvars` added `Instructions<T>` and `SlotHashes<T>` support to `Sysvar<T>`.
- `3fda2486 v2: Improve support for more seed expressions in IDL output (#4489)` changed PDA seed IDL emission to handle more const-evaluable seed expressions.

New CPI codegen details:

- `#[derive(Accounts)]` now emits a sibling `__cpi_accounts_<accounts>` module for supported account structs.
- The generated CPI account struct has one `CpiHandle<'a>` field per account and implements `ToCpiAccounts<'a>`.
- `#[program]` re-exports these under `program::cpi::accounts::<Accounts>` behind the `cpi` feature.
- `#[program]` also emits wrapper functions like `callee::cpi::set_data(cpi_ctx, value)` that pack instruction args and call `CpiContext::invoke`.
- This is currently skipped for `Accounts` structs containing `Option<_>` or `Nested<_>`, because those require more flattening/fallback logic.
- The generated `InstructionAccount` flags come from compile-time writable/signer metadata and cover all four cases: writable signer, writable non-signer, readonly signer, readonly non-signer.
- Tests in `tests-v2/tests/cpi.rs` cover no-arg CPI wrappers, duplicate accounts re-export dedupe, all account-flag combinations, wrong-authority failure propagation, and wrong-program rejection.

IDL seed emission details after `3fda2486`:

- Runtime validation supports both array-form seeds (`seeds = [..]`) and expression-form seeds (`seeds = tag_seeds()` / `seeds = CONST_SEEDS`).
- Client-side PDA auto-derivation and generated `find_<field>_address` helpers only work for array-form seeds the macro can inspect.
- IDL emission for array-form seeds classifies:
  - byte/string/byte-array literals as `{"kind":"const","value":[...]}`;
  - account roots like `user`, `user.address().as_ref()`, `user.key().as_ref()` as `{"kind":"account","path":"user"}`;
  - instruction arg roots as `{"kind":"arg","path":"arg"}`;
  - other const-evaluable expressions like `MY_PREFIX`, `crate::seeds::TAG`, or `System::id()` as runtime calls to `anchor_lang_v2::idl_build::__idl_const_seed_json(expr)`.
- `__idl_const_seed_json` accepts `impl AsRef<[u8]>` and renders real byte arrays in the IDL. If an expression does not implement `AsRef<[u8]>`, the IDL build should fail instead of silently emitting an empty/broken seed.
- Non-array seed expressions still surface as a placeholder `{"kind":"expr"}` in the IDL.

Constraint lifecycle nuance:

- Namespaced constraints dispatch through `AccountConstraint<A>`:
  - `ns::key = v` -> `check`.
  - `init, ns::key = v` -> `init`.
  - `init_if_needed, ns::key = v` on create -> `init`, then `check`.
  - `init_if_needed, ns::key = v` on existing account -> `check`.
  - `update(ns::key = v)` -> `update`.
  - any of the above -> `exit` during successful account exit.
- There is deliberately no blanket `AccountConstraint<Option<A>>`; the derive emits inline `if let Some(inner)` routing for optional fields.
- `Box<T>` does forward `AccountConstraint<Box<T>>` to the inner `T`.

Potential issue noticed:

- `lang-v2/src/accounts/sysvar.rs` sets `SysvarId for pinocchio::sysvars::instructions::Instructions<T>` to `INSTRUCTIONS_ID` at runtime, but its `IDL_ADDRESS` string is currently `"SysvarC1ock11111111111111111111111111111111"`, same as Clock. That looks likely wrong for IDL emission; expected address is probably the instructions sysvar address. Verify before filing/fixing.

Docs-code freshness note:

- `docs-v2/src/content/docs/v2/cpi.mdx` already explains `CpiContext`, `CpiHandle`, unchecked CPI, and `Resolved`.
- After `528b790d`, it should probably also explicitly document generated `program::cpi::accounts::*` structs and `program::cpi::<ix>(ctx, args...)` wrappers for user programs, plus the current `Option<_>` / `Nested<_>` unsupported caveat.

## Docs IA Draft: Versioned v1/v2 Structure

Goal from user: current docs are v1 docs with v2 alpha shoved near the bottom. Target should be two first-class doc versions, v1 and v2. They should functionally rhyme, but not force v2 into exact v1 shapes or preserve the current v1 structure if it can be improved.

Recommended route shape:

- `/docs/` = version-aware landing page or default redirect with clear "Stable v1" and "Alpha v2" choices.
- `/docs/v1/...` = stable/current v1 docs.
- `/docs/v2/...` = alpha v2 docs.
- `/docs/project/...` or `/docs/updates/...` = cross-version project material: release notes, changelog, contribution guide.
- Add a version switcher in the docs chrome; do not keep v2 as a normal sidebar item under v1.

Shared high-level sections for both versions:

1. Overview
2. Get started
3. Fundamentals
4. Program development
5. Clients and IDL
6. Tokens and CPI
7. Testing and debugging
8. Security and production
9. Reference

Draft v1 tree:

- `v1/index` - What Anchor is, stable status, when to use v1 vs v2.
- `v1/get-started/installation` - AVM, CLI install, Solana toolchain.
- `v1/get-started/quickstart` - default beginner path.
- `v1/get-started/local-development` - local validator/workspace loop.
- `v1/get-started/solana-playground` - browser path.
- `v1/fundamentals/program-structure` - `#[program]`, modules, `declare_id!`.
- `v1/fundamentals/accounts-and-context` - `Context<'info, T>`, `AccountInfo`, `ctx.accounts`, `ctx.bumps`.
- `v1/fundamentals/account-validation` - `#[derive(Accounts)]`, built-in constraints, `has_one`, `seeds`, `init`.
- `v1/fundamentals/pdas` - seeds, bumps, `find_program_address`, account resolution.
- `v1/fundamentals/idl` - generated IDL and how clients use it.
- `v1/fundamentals/cpi` - `CpiContext`, `ToAccountInfos`, CPI helpers.
- `v1/programs/account-types` - `Account`, `AccountLoader`, `Program`, `Signer`, `SystemAccount`, `UncheckedAccount`, `InterfaceAccount`.
- `v1/programs/account-space-and-realloc` - `InitSpace`, `space`, realloc, close.
- `v1/programs/errors` - `#[error_code]`, `require!`.
- `v1/programs/events` - `emit!`, `emit_cpi!`.
- `v1/programs/zero-copy` - v1-specific zero-copy via `AccountLoader`.
- `v1/clients/typescript` - generated TS client and methods builder.
- `v1/clients/rust` - Rust client.
- `v1/clients/declare-program` - dependency-free composability.
- `v1/tokens/spl-token-basics` - mint/account/transfer/mint-to.
- `v1/tokens/token-2022-and-extensions` - extensions/interface accounts.
- `v1/testing/anchor-test` - workspace tests, local validator.
- `v1/testing/litesvm`
- `v1/testing/mollusk`
- `v1/security/sealevel-attacks`
- `v1/security/footguns`
- `v1/security/verifiable-builds`
- `v1/reference/macros-and-attributes`
- `v1/reference/account-constraints`
- `v1/reference/anchor-toml`
- `v1/reference/cli`
- `v1/reference/avm`
- `v1/reference/rust-to-js-types`
- `v1/reference/examples`

Draft v2 tree:

- `v2/index` - Alpha status, design goals, headline perf/security/extensibility story.
- `v2/get-started/installation` - git install from `anchor-next`, avm/path warning, macOS LTO workaround.
- `v2/get-started/quickstart` - minimal counter with `anchor-lang-v2`.
- `v2/get-started/migrating-from-v1` - rename table and migration strategy; prominent, not buried in reference.
- `v2/fundamentals/program-structure` - `#![no_std]`, `#[program]`, `&mut Context<T>`, instruction discriminators.
- `v2/fundamentals/accounts-and-context` - no `<'info>`, `Context`, typed bumps, lazy remaining accounts.
- `v2/fundamentals/account-validation` - `#[derive(Accounts)]`, constraints, duplicate-mut mask, `unsafe(dup)`, `address` over `has_one`.
- `v2/fundamentals/pdas-and-resolution` - seeds, bump precompute, `Resolved`, generated PDA helpers, IDL seed limits.
- `v2/fundamentals/idl` - v2 IDL emission, type deps, serialization metadata, account resolution metadata.
- `v2/fundamentals/cpi` - `CpiContext`, `CpiHandle`, generated `program::cpi::*` wrappers, unchecked CPI safety, limitations for `Option`/`Nested`.
- `v2/programs/account-data-model` - zero-copy default mental model: `Account<T>`, discriminator + repr(C) data.
- `v2/programs/account-types` - wrappers: `Account`, `BorshAccount`, `Slab`, `Nested`, `Option<Account<T>>`, `Box<T>`, `Sysvar<T>`.
- `v2/programs/pod-types` - `PodU*`, `PodBool`, `PodVec`, `#[pod_wrapper]`.
- `v2/programs/borsh-accounts-and-realloc` - variable-length data, `release_borrow`, `reacquire_borrow_mut`, realloc caveats.
- `v2/programs/errors-and-require`
- `v2/programs/events` - wincode default and `#[event(bytemuck)]`.
- `v2/clients/typescript` - likely alpha/incomplete until package story lands.
- `v2/clients/rust` - generated instruction/account helpers and `Resolved`.
- `v2/tokens/spl-token-basics` - `anchor-spl-v2` CPI helpers.
- `v2/tokens/token-2022-and-extensions` - `InterfaceAccount`, TLV extensions.
- `v2/testing/anchor-v2-testing` - LiteSVM wrapper and re-exports.
- `v2/testing/profile-and-flamegraphs` - `anchor test --profile`.
- `v2/testing/debugger` - TUI and `--gdb`.
- `v2/testing/coverage` - `anchor coverage`.
- `v2/security/secure-by-default` - compile-time footgun removal, unchecked CPI safety story, duplicate-mut guard.
- `v2/security/production-builds` - guardrails/account-resize/const-rent tradeoffs, audit warning.
- `v2/advanced/optimizations` - why binaries/CUs shrink.
- `v2/advanced/extensibility` - `AnchorAccount`, `AccountConstraint`, `Id`, `Discriminator`.
- `v2/advanced/custom-account-types`
- `v2/advanced/custom-constraints`
- `v2/advanced/asm-v2` - if the asm helper crate should be documented publicly.
- `v2/reference/macros-and-attributes`
- `v2/reference/account-constraints`
- `v2/reference/account-types`
- `v2/reference/feature-flags`
- `v2/reference/anchor-toml`
- `v2/reference/cli`
- `v2/reference/idl`
- `v2/reference/examples-and-benchmarks`

Content move notes:

- Current root `basics/*`, `clients/*`, `testing/*`, `features/*`, `tokens/*`, and `references/*` become v1 material, but reorganized into the tree above instead of copied as-is.
- Current `v2/*` pages become seeds for the full v2 tree. Split the current large pages where useful:
  - `v2/account-types.mdx` -> account data model, account types, borsh/realloc.
  - `v2/macros.mdx` -> fundamentals/program-structure plus reference/macros.
  - `v2/cpi.mdx` -> fundamentals/cpi, updated for generated CPI wrappers.
  - `v2/optimizations.mdx` -> advanced/optimizations.
  - `v2/extensibility.mdx` -> advanced/extensibility plus custom account/constraint how-tos.
- Keep `updates` global/cross-version unless v2 alpha gets its own changelog.

## Docs Restructure Progress: 2026-04-30

Completed first structural pass for v1:

- Added a new `/docs/` landing page that links to `/docs/v1/` and `/docs/v2/`.
- Added root `_meta.ts` with `Docs home`, `Anchor v1`, current `Anchor v2 (alpha)`, and `updates`.
- Moved existing v1 docs into:
  - `v1/get-started`
  - `v1/fundamentals`
  - `v1/programs`
  - `v1/clients`
  - `v1/tokens`
  - `v1/testing`
  - `v1/security`
  - `v1/reference`
- Kept current `v2/*` pages in their existing compact sidebar section for now.
- Moved all raw example/helper fixture folders along with their MDX pages (`_cpi`, `_idl`, `_pda`, `_declare-program`, `_errors`, token examples, client examples, etc.).
- Updated internal content links from old root v1 paths to `/docs/v1/...`; public image paths like `/docs/quickstart/*.png` intentionally remain unchanged.
- `bun run build` in `docs-v2/` passes. Remaining output is pre-existing hints/warnings about deprecated `navigator.platform` and font URLs left to resolve at runtime.
