# anchor-lang-v2

v2 is a drop-in speedup for Anchor v1. Same constraints DSL, same `#[derive(Accounts)]` — but up to **94% smaller** and **3–4× faster** per instruction (see the [Examples](#examples) table). v2 also comes with first-class tooling like `anchor debugger` to help you optimize your programs.

**Compatibility is a priority.** Most v1 programs port with the renames in [Migrating from v1](#migrating-from-v1). A `compat` feature restores v1-shaped helpers to ease migrations for larger programs.

**Built for extensibility.** As active contributors and auditors of Anchor v1, we've felt the struggles of working inside a large macro-based framework — v2 is our answer. The core derive is ~3,700 lines, down from ~11,400 in v1 (roughly a third the size), because most of the logic that used to live *inside* the macro now lives *behind traits*. An exciting implication is you can write your own `Account<T>`, `LazyAccount<T>`, or whatever your heart desires *without* touching the core audited framework (see [Extensibility](#extensibility)).

**And most importantly, v2 is secure by default for users.** Safer APIs steer you away from common v1 footguns, promoting whole classes of runtime bugs into compile errors. [Fuzzing](https://github.com/asymmetric-research/crucible-harnesses), static analysis, and formal verification are builtin with first-class support.

> [!WARNING]
> **Alpha.** Not audited, not on crates.io. APIs may break between commits. Depend via git on the [`anchor-next`](https://github.com/solana-foundation/anchor/tree/anchor-next) branch.

## Examples

Worked programs live under [`bench/programs/`](../bench/programs/), paired with v1 for a head-to-head comparison. Binary sizes and CU numbers are from the v2 variant.

| Program | Description | Binary | CU range | vs v1 |
|---|---|---|---|---|
| [helloworld](../bench/programs/helloworld/anchor-v2) | Single-instruction counter | 6.9 KB | 1,383 | −94% bin / −76% CU |
| [prop-amm](../bench/programs/prop-amm/anchor-v2) | Oracle feed with asm fast-path | 9.2 KB | 26–1,383 | −93% bin / −68% to −98% CU |
| [vault](../bench/programs/vault/anchor-v2) | Single-depositor SOL vault | 5.9 KB | 403–1,910 | −94% bin / −67% to −84% CU |
| [nested](../bench/programs/nested/anchor-v2) | `Nested<T>` shared-validation demo | 13 KB | 476–2,748 | −91% bin / −86% to −90% CU |
| [multisig](../bench/programs/multisig/anchor-v2) | Four-instruction SOL multisig | 31 KB | 477–2,363 | −81% bin / −67% to −89% CU |

## Getting started

```bash
$ cargo install --git https://github.com/solana-foundation/anchor.git --branch anchor-next anchor-cli --force
$ anchor init --no-install counter && cd counter
$ anchor build && anchor test
$ anchor debugger                    # optional: step through the SBF trace in a TUI
```

> [!NOTE]
> `cargo install` overwrites the `anchor` binary on your `PATH`, including one from [`avm`](https://www.anchor-lang.com/docs/installation#anchor-version-manager-avm). To keep v1 alongside, run the debug binary directly from a source checkout. `--no-install` skips the TS deps until `@anchor-lang/core@2.0.0-rc.N` ships — tests are pure Rust (LiteSVM), so the TS client isn't needed for the default workflow.

> [!NOTE]
> On macOS you may hit `ld: could not parse bitcode object file … Unknown attribute kind` during the final link. That's an LLVM-version skew between rustc's bitcode and Apple's system `libLTO`. Turn LTO off for the install:
>
> ```bash
> $ CARGO_PROFILE_RELEASE_LTO=off cargo install --git https://github.com/solana-foundation/anchor.git --branch anchor-next anchor-cli --force
> ```

## Migrating from v1

Most v1 programs port with the renames below. `#[derive(Accounts)]` constraints, `emit!`, `#[error_code]`, `require!`, `CpiContext`, and the TS `program.methods.foo(...).accounts(...).rpc()` entry point all still work as-is.

| v1 | v2 |
|---|---|
| `solana_program` | `pinocchio` |
| `std` required | `#![no_std]` (`alloc` is a default feature) |
| `<'info>` everywhere | removed — pinocchio's account model is static-scoped |
| `Context<T>` in handlers | `&mut Context<T>` (constraints + exit hooks mutate context state) |
| `.key()` on accounts | `.address()` |
| `Pubkey` | `Address` (drop-in replacement — same 32-byte type) |
| `Account<T>` defaults to borsh | `Account<T>` defaults to pod; v1 semantics → `BorshAccount<T>` |
| `space = 8 + T::INIT_SPACE` required on `init` | optional for pod `Account<T>` — defaults to `8 + size_of::<T>()` |
| `Pubkey::find_program_address` | `find_program_address` re-exported from crate root |

### `compat` feature

Off by default. Adds v1-shaped helpers that aren't `no_std`-friendly. Today that's `debug!` — a `msg!`-shaped macro that accepts any Rust format string (`{:?}`, `{:x}`, dynamic width) via `alloc::format!`. It heap-allocates, so prefer `msg!` on hot paths; `compat` exists for ports that rely on the wider formatting surface.

```toml
[dependencies]
anchor-lang-v2 = { git = "...", branch = "anchor-next", features = ["compat"] }
```

Other features, default-on and usually left alone: `alloc` (bump allocator), `guardrails` (runtime borrow-state checks, ~0–5 CU per ix), `account-resize` (`realloc_account`). Turn them off with `default-features = false` if you're chasing CUs. `const-rent` is opt-in: bakes the rent formula into the binary and saves ~85 CU per `create_account`.

## Optimizations

Concrete CU / binary wins v2 carries on top of pinocchio.

| Trick | Savings |
|---|---|
| **PDA bumps precomputed at macro time.** If your seeds are all literals, the derive runs the PDA search during compilation and bakes the canonical bump in as a `const` (`derive/src/pda.rs`). | Skips the ~255-iteration runtime PDA loop |
| **Skip the on-curve check for program-owned PDAs.** If the program already owns the account, it had to be created via signed CPI — which did the curve check at the time. Verification can just hash-and-compare (`src/cpi.rs:220`). | ~1,000 CU per verify |
| **Wincode events by default.** Much cheaper than borsh on SBF, and still handles `Vec` / `String` / `Option` / enums (`src/event.rs`). | 3–10× vs borsh |
| **`#[event(bytemuck)]` for fixed-size events.** The struct's `repr(C)` Pod layout already matches the wire format, so emitting is just disc + one memcpy of the body (`src/event.rs`). | No per-field encoding |
| **Alignment-1 Pod wrappers** (`PodU64`, `PodI128`, `PodBool`, ...). Integers stored as `[u8; N]` so the whole `#[account]` struct casts directly from the account's raw bytes (`src/pod.rs`). | Zero deserialization |
| **`PodVec<T, MAX>`**: fixed-capacity vec with a `u16` length, stored inline in the account (`src/pod.rs:546`). | Variable length without heap |
| **Typed `CpiHandle` lets us use pinocchio's unchecked CPI.** The unchecked path would be UB under stale-borrow aliasing, but the Rust borrow checker rules that out at compile time — so `CpiContext::invoke()` takes it (`src/context_cpi.rs:75`). | UB → compile error, one fewer runtime check per CPI |
| **`const-rent` feature** bakes the rent formula into the binary so `create_account` skips the `Rent::get()` sysvar (`src/cpi.rs:14`). | ~85 CU per `create_account` |
| **Guardrails compile away** when you drop the feature. `check_program_id` / `check_max_accounts` / the `is_writable` check in `load_mut` just aren't emitted (`src/lib.rs:257`). | ~0 CU + smaller binary in prod |

## Account types

No `'info` lifetime on any of these.

| Type | Use | Notes |
|---|---|---|
| `Account<T>` | **Default for your program's data.** | Zero-copy. Requires `T: Pod` (enforced by `#[account]`). Field access is a single deref on a cached typed pointer. Layout: `[8-byte disc][repr(C) T]`. |
| `BorshAccount<T>` | Data with `Vec` / `String` / enums. | Deserializes on load, serializes on exit. |
| `Signer` | Must be `is_signer`. | Cheap. |
| `Program<T>` | CPI targets (`Program<System>`, `Program<Token>`, …). | Validates executable + program ID via `T: Id`. |
| `SystemAccount` | System-owned payload. | Owner check only. |
| `UncheckedAccount` | Escape hatch. | No validation. |
| `Sysvar<T>` | `Sysvar<Clock>`, `Sysvar<Rent>`. | Prefer `Clock::get()` / `Rent::get()` syscalls where possible. |
| `Slab<H, Item>` | Header + dynamic item tail. | Zero-copy ledger / event-log accounts. `Account<T>` is `Slab<T, HeaderOnly>` under the hood. |
| `Option<Account<T>>` | Optional slot. | Client sends program-ID as sentinel when absent; bumps become `Option<u8>`. |
| `Nested<T>` | Compose `#[derive(Accounts)]` structs. | Inline expansion. Access via `ctx.accounts.inner.field`. |

**Why `Account<T>` is pod by default.** Stored bytes match the struct layout, so load is a pointer cast and exit is a no-op — no (de)serialization, no heap. `#[account]` enforces `T: Pod` per field, so `Vec` / `String` / `Option` / `bool` fail at compile time. `u128` / `i128` fields need `PodU128` / `PodI128` (host is align-16, SBF is align-8; the wrappers guarantee align-1 on both).

Use `BorshAccount<T>` when you need dynamic-size fields, or `Slab<H, Item>` when the dynamic part is a homogeneous array.

## Macros

- **`#[program]`** / **`#[derive(Accounts)]`** / **`#[account]`** / **`#[account(borsh)]`** — same surface as v1, with the delta that handlers take `&mut Context<T>` and `#[account]` defaults to pod.
- **`#[error_code]`** / **`#[error_code(offset = N)]`** — per-program error enum, variants map to `ProgramError::Custom(n)`, `#[msg("...")]` attaches a human message. Multiple enums per program are supported.
- **`#[constant]`** — marks a `pub const` as IDL-visible.
- **`#[access_control(fn1(ctx), fn2(ctx))]`** — prepends `?`-propagating calls before the handler body.
- **`#[derive(InitSpace)]`** — computes `Space::INIT_SPACE` for `BorshAccount` types. Handles primitives, `Address`, `String` + `#[max_len(N)]`, `Vec<T>` + `#[max_len(N)]`, `Option<T>`, arrays, tuples, nested derives. Pod `#[account]` doesn't need it — `Account<T>::INIT_SPACE` defaults to `8 + size_of::<T>()`.
- **`#[event]` / `#[event(bytemuck)]` / `#[event(borsh)]`** — event emission via `emit!`. Default is wincode (3–10× cheaper than borsh on SBF, supports `Vec`/`String`/`Option<T>`/enums). `bytemuck` is a zero-copy memcpy for `repr(C)` Pod structs. `borsh` preserves wire compatibility with v1 off-chain consumers.
- **`require!` family** — `require!` / `require_eq!` / `require_neq!` / `require_keys_eq!` / `require_keys_neq!` / `require_gt!` / `require_gte!` desugar to `return Err(err.into())`. `#[error_code]` enums `impl From<E> for Error`; raw `ProgramError::*` variants work too.

## CPI and ix construction

Same `CpiContext` shape as v1. Three structural improvements on top:

- **Typed `cpi_handle()` / `cpi_handle_mut()`.** Each handle is a Rust borrow of the account (shared or exclusive), so the borrow checker catches at compile time what v1 caught at runtime with an `AccountBorrowFailed` panic. This matters for more than error quality: `CpiContext::invoke()` uses pinocchio's `invoke_signed_unchecked` for the CU win, which skips the runtime borrow check — without the typed handle gating it, the same aliasing pattern would be UB.
- **Generated `cpi::accounts::<Handler>` structs.** Fill accounts in by name, not by building `AccountMeta` vectors.
- **Client-side `<Accounts>Resolved` structs.** Alongside the full `accounts::<Handler>` (every field an `Address`), the derive emits a `<Handler>Resolved` variant with only the fields the caller has to provide. `system_program` / `token_program` / PDAs get auto-filled in `to_account_metas()` — PDAs derive in topological order so dependent seeds work.

```rust
use anchor_spl_v2::{token, TokenAccount};

pub fn transfer(ctx: &mut Context<Transfer>, amount: u64) -> Result<()> {
    let cpi_accounts = token::cpi::accounts::Transfer {
        from: ctx.accounts.from.cpi_handle_mut(),
        to: ctx.accounts.to.cpi_handle_mut(),
        authority: ctx.accounts.authority.cpi_handle(),
    };
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.account().address(),
        cpi_accounts,
    );
    token::cpi::transfer(cpi_ctx, amount)?;
    Ok(())
}
```

Use `CpiContext::new_with_signer(program, accounts, seeds)` (or `.with_signer(seeds)`) for PDA-signed CPIs.

`Resolved` cuts boilerplate in tests and SDK callers. In the multisig bench, `Create` takes `creator` (signer), `config` (PDA from `[b"multisig", creator]`), and `system_program`. The caller only passes `creator`:

```rust
// multisig_v2::accounts::Create { creator, config, system_program }   // full — all three
// multisig_v2::accounts::CreateResolved { creator }                    // resolved — just the input

let metas = multisig_v2::accounts::CreateResolved { creator: creator.pubkey() }
    .to_account_metas(None);   // auto-derives `config` PDA, auto-fills `system_program`
```

If you need the PDA somewhere else, every seed-bearing field also gets a finder method on the full struct — `Create::find_config_address(&creator)`.

Lower-level helpers if you're not using `CpiContext`:

```rust
create_account(&payer, &new_account, space, &owner_program)?;
create_account_signed(&payer, &new_account, space, &owner_program,
    &[b"vault", user.address().as_ref(), &[bump]])?;

let (address, bump) = find_program_address(&[b"vault", user.address().as_ref()], program_id);
let bump = verify_program_address(&[b"vault", ...], program_id, &expected_addr)?;
let bump = find_and_verify_program_address_skip_curve(&[b"vault", ...], program_id, &expected_addr)?;
```

## Patterns

- **PDA with stored bump.** Store the bump if your program later signs CPIs with the PDA. If every instruction re-validates via `#[account(seeds = [...], bump)]`, storing is optimization, not correctness.
- **`remaining_accounts()` is lazy-cached.** First call walks the cursor; subsequent calls clone the cached vec. Validate count and owner before dispatching into a CPI.
- **Handler-body validation order.** Constraints run in field order. Inside the handler, order by security criticality: signatures / authorization → `has_one` / membership → state machine → timelock → mutation.

## Tooling

Both tools run in-process on LiteSVM. No validator, no network.

**`anchor test --profile`** rebuilds with DWARF, runs the tests, captures SBF register traces, and writes a flamegraph SVG per tx to `target/anchor-v2-profile/`. The scaffold wires up the `profile` Cargo feature that forwards to `anchor-v2-testing/profile`; `--profile` turns it on for you.

**`anchor debugger`** is a Foundry-style instruction-level TUI on top of the same traces. Source-mapped: Rust next to SBF disasm, per-instruction CU.

```bash
$ anchor debugger                       # build + test + open TUI
$ anchor debugger my_test_name          # filter to a specific test
$ anchor debugger --skip-run            # open over existing traces
```

## Extensibility

An important implication of our trait-based framework is: **you can write your own Anchor extensions.**

In v1, anything the macro didn't already support meant forking the derive. v2 moves most logic out of the macro and behind traits, so anyone can ship new behavior from a separate crate — no fork, no upstream PR. The core derive shrinks from ~11,400 LoC to ~3,700 as a nice side effect.

For example, [anchor-dynamic-account](https://github.com/chen-robert/anchor-dynamic-account) adds a brand-new primitive — zero-copy accounts with a `Vec<T>` / `String` tail that auto-reallocates to fit — behind a single `#[wrapped_account]` macro, with no changes to `anchor-lang-v2`:

```rust
#[wrapped_account]
pub struct Post {
    pub author: Address,
    pub body:   Vec<u8>,     // tail, auto-reallocs to fit
}

#[derive(Accounts)]
pub struct Edit {
    #[account(mut)]
    pub author: Signer,
    #[account(mut, dynamic_account::payer = author)]
    pub post: DynamicAccount<Post>,
}
```

At the call site, `DynamicAccount<T>` is a cosmetic alias that reads parallel to v2's `Account<T>` / `BorshAccount<T>`. Under the hood, the macro plugs it into v2 by implementing `AnchorAccount`:

```rust
impl AnchorAccount for DynamicAccount<Post> {
    type Data = PostFixed;
    fn load(view, pid)   -> Result<Self>     { /* parse disc + tail */ }
    fn exit(&mut self)   -> ProgramResult    { /* realloc to fit, persist */ }
    // load_mut, load_mut_after_init, account
}
```

## Contributing

File issues at [solana-foundation/anchor](https://github.com/solana-foundation/anchor/issues), tagged with `v2` where applicable. Working branch: `anchor-next`. See the top-level [CONTRIBUTING.md](../CONTRIBUTING.md).
