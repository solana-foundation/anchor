# anchor-lang-v2

Next-generation [Anchor](https://www.anchor-lang.com/) runtime for Solana programs. Built on [pinocchio](https://github.com/anza-xyz/pinocchio), `#![no_std]` by default, trait-based account system. Drops the `'info` lifetime from user-facing code.

Order-of-magnitude smaller binaries and fewer CU per instruction versus v1 ([bench results](../bench/)):

| Program | v1 | v2 | Δ binary | Δ CU (median ix) |
|---|---|---|---|---|
| helloworld (1 ix) | 122 KB / 5,855 CU | 6.9 KB / 1,383 CU | **−94%** | **−76%** |
| multisig (4 ix) | 166 KB / 4,324–12,031 CU | 31 KB / 476–2,360 CU | **−81%** | **−67%** |
| vault (2 ix) | — | 5.9 KB / 403–1,910 CU | — | — |
| prop-amm (3 ix) | — | 9.2 KB / 26–1,383 CU | — | — |

> [!WARNING]
> **Alpha — do not use in production.** Not audited. APIs may break between commits. Programs written against today's `anchor-lang-v2` are not guaranteed an upgrade path. Not published to crates.io; depend via git on the [`anchor-next`](https://github.com/solana-foundation/anchor/tree/anchor-next) branch.

## Quick start

> [!NOTE]
> The install below overwrites the `anchor` binary on your `PATH` — including one installed by [`avm`](https://www.anchor-lang.com/docs/installation#anchor-version-manager-avm). To keep an `avm`-managed install alongside, clone the repo and run v2 out of `./target/debug/anchor` directly.

```bash
cargo install --git https://github.com/solana-foundation/anchor.git \
  --branch anchor-next anchor-cli
anchor init counter
cd counter
anchor build
anchor test
```

> [!NOTE]
> The default test template is **LiteSVM** — tests run as pure Rust
> (`cargo test`), so the usual v2 workflow never touches the TS client.
> The scaffold still emits a `package.json` at init time, and the
> `yarn install` it triggers pins `"@anchor-lang/core": "^<cli-version>"`
> (e.g. `^2.0.0`), a version not yet on npm — the install step will fail
> until `2.0.0-rc.N` ships. Workarounds for those who want to get past it:
> - run `anchor init --no-install` and skip the TS deps entirely, or
> - downgrade the pin to `^1.0.0` in `package.json` before running
>   `yarn install`, or
> - link the local TS package:
>   `yarn add file:../../anchor/ts/packages/anchor`.
>
> Tracked in `cli/src/rust_template.rs` above both `package_json` /
> `ts_package_json` templates. The long-term fix is publishing an rc —
> npm doesn't support subdirectory git deps, so pinning to the in-repo
> TS package via a git URL isn't viable.

The scaffold generates a minimal program that initializes a single account:

```rust
use anchor_lang_v2::prelude::*;

declare_id!("...");

#[program]
pub mod counter {
    use super::*;

    pub fn initialize(ctx: &mut Context<Initialize>) -> Result<()> {
        ctx.accounts.counter.count = 0;
        ctx.accounts.counter.authority = *ctx.accounts.payer.address();
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {
    #[account(mut)]
    pub payer: Signer,
    #[account(init, payer = payer)]
    pub counter: Account<Counter>,
    pub system_program: Program<System>,
}

#[account]
pub struct Counter {
    pub count: u64,
    pub authority: Address,
}
```

Two things that trip v1 users:

- **No `<'info>` lifetime** on the `Accounts` struct or its fields. Pinocchio's account model is static-scoped; the derive handles it.
- **Handlers take `&mut Context<T>`.** Not `Context<T>`. The `&mut` is load-bearing — constraint validation and exit hooks need to mutate state on the context.

The generated test lives in `tests/test_initialize.rs` and runs via [LiteSVM](https://github.com/LiteSVM/litesvm) in-process (`cargo test`, no validator).

## Core concepts

**`#[program]`** — the instruction-dispatch module. Each `pub fn` is a handler taking `&mut Context<T>` plus deserialized args (wincode-encoded).

**`#[derive(Accounts)]`** — declares accounts + constraints. All validation runs before your handler. Field order matches the account order in the instruction.

**`#[account]`** — pod-mode data account by default: zero-copy cast, `repr(C)`, auto `bytemuck::Pod`/`Zeroable`, no-padding compile check. Add `(borsh)` for dynamic-size types.

**Discriminator** — first 8 bytes of every account/instruction/event data. Computed as `sha256("account:Name")[..8]` / `"global:Name"` / `"event:Name"` respectively, at macro-expansion time.

**`Context<T>`** — carries `accounts` (validated), `bumps` (PDA bump values from `seeds = ...`), `program_id`, and lazy `remaining_accounts()`.

## Account types

All types are bare — no `'info` lifetime.

| Type | Use | Notes |
|---|---|---|
| `Account<T>` | **Default for your program's data.** | Zero-copy. Requires `T: Pod` (enforced by `#[account]`). Field access is a single dereference on a cached typed pointer. Layout: `[8-byte disc][repr(C) T]`. |
| `BorshAccount<T>` | Your data with dynamic fields (`Vec`, `String`, enums). | Deserializes on load, serializes on exit. Real cost per-instruction. Use when `Pod` is impossible. |
| `Signer` | Must have `is_signer = true`. | Cheap. |
| `Program<T>` | CPI targets (`Program<System>`, `Program<Token>`, etc.). | Validates executable + program ID via `T: Id`. |
| `SystemAccount` | System-owned payload. | Owner check only. |
| `UncheckedAccount` | Escape hatch. | No validation. Use only when other types can't express what you need. |
| `Sysvar<T>` | `Sysvar<Clock>`, `Sysvar<Rent>`. | Address validated against the sysvar ID. Prefer `Clock::get()` / `Rent::get()` syscalls where possible. |
| `Slab<H, Item>` | Header + dynamic item tail. | Zero-copy ledger / event-log accounts. `Account<T>` is `Slab<T, HeaderOnly>` under the hood. |
| `Option<Account<T>>` | Optional account slot. | Client sends program-ID as sentinel when absent; constraints are skipped; bumps become `Option<u8>`. |
| `Nested<T>` | Compose `#[derive(Accounts)]` structs. | Inline expansion — `Nested<Inner>` consumes Inner's accounts at that position in the flat list. Access nested fields via `ctx.accounts.inner.field` (auto-deref). |

### Why `Account<T>` is the default

- **Field access is a pointer deref.** Cached typed pointer at load time, no per-field overhead.
- **No (de)serialization pass.** Stored bytes already match the struct layout. Exit is a no-op for `Account<T>`; `BorshAccount<T>` has to serialize on exit.
- **No heap.** Pod types sit in-place in the account's data buffer.
- **Compile-time guards.** The `#[account]` macro enforces `T: Pod` per field (catches `Vec`/`String`/`Option`/`bool` even inside user structs) plus an unconditional no-padding assertion. Fat pointers inside user types can't sneak through. Because the padding check runs on every target, raw `u128`/`i128` fields need the alignment-1 `PodU128`/`PodI128` wrappers — see the Pod wrappers pattern below.

Reach for `BorshAccount<T>` when data genuinely needs dynamic-size fields. Reach for `Slab<H, Item>` when the dynamic part is a homogeneous array (ledger entries, per-block events, etc.).

## Constraints

```rust
#[derive(Accounts)]
pub struct Example {
    #[account(mut)]
    pub data: Account<Data>,

    // `space =` is optional for pod accounts — defaults to 8 + size_of::<Data>().
    #[account(init, payer = payer)]
    pub new_data: Account<Data>,

    // BorshAccount: specify size explicitly, or derive InitSpace.
    #[account(init, payer = payer, space = 8 + Profile::INIT_SPACE)]
    pub profile: BorshAccount<Profile>,

    #[account(has_one = authority)]
    pub managed: Account<Managed>,
    pub authority: Signer,

    #[account(mut, seeds = [b"vault", payer.address().as_ref()], bump)]
    pub vault: Account<Vault>,  // bump available at ctx.bumps.vault

    #[account(constraint = config.enabled != 0 @ MyError::Disabled)]
    pub config: Account<Config>,  // `@` attaches a custom error code

    #[account(close = recipient)]
    pub closeable: Account<Data>,
    /// CHECK: receives rent from the closed account.
    #[account(mut)]
    pub recipient: UncheckedAccount,

    #[account(mut)]
    pub payer: Signer,

    pub system_program: Program<System>,
}
```

## Macros

### `#[error_code]`

Per-program error enum. Each variant maps to a `ProgramError::Custom(n)`; `#[msg("...")]` attaches a human message.

```rust
#[error_code]
pub enum MyError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Stale counter")]
    Stale,
}

// require!(ok, MyError::Unauthorized);
```

### `#[constant]`

Marks a `pub const` as IDL-visible. Values must be const expressions.

```rust
#[constant]
pub const SEED: &str = "anchor";
```

### `#[access_control(fn1(a), fn2(b, c))]`

Prepends `?`-propagating calls to the listed fns before your handler body. Authorization gates go here.

```rust
#[access_control(verify_epoch(ctx))]
pub fn vote(ctx: &mut Context<Vote>) -> Result<()> { /* ... */ }
```

### `#[derive(InitSpace)]`

Computes `Space::INIT_SPACE` from field types. Required on `BorshAccount` types when you want `space = 8 + T::INIT_SPACE`. Handles primitives, `Address`, `String`+`#[max_len(N)]`, `Vec<T>`+`#[max_len(N)]`, `Option<T>`, arrays, tuples, nested derives.

```rust
#[account(borsh)]
#[derive(InitSpace)]
pub struct Profile {
    pub authority: Address,
    #[max_len(32)]
    pub name: String,
    #[max_len(8)]
    pub friends: Vec<Address>,
}
```

Pod `#[account]` doesn't need `InitSpace` — the `Account<T>::INIT_SPACE` fallback computes `8 + size_of::<T>()` directly.

### `#[event]` / `#[event(bytemuck)]` / `#[event(borsh)]`

Event definitions emitted via `emit!`. Three serialization modes:

- **Default `#[event]`**: wincode serialization. Supports `Vec`, `String`, `Option<T>`, enums — and is 3–10× cheaper than borsh on SBF.
- **`#[event(bytemuck)]`**: zero-copy `copy_nonoverlapping` of a `repr(C)` Pod struct. Cheapest for fixed-size shapes, but rejects fat-pointer and non-Pod fields at compile time. No padding allowed.
- **`#[event(borsh)]`**: borsh serialization. Retained for IDL-compatibility with v1 off-chain consumers that decode via borsh.

```rust
#[event]
pub struct Deposited {
    pub depositor: Address,
    pub amount: u64,
}

#[event(bytemuck)]
pub struct Tick {
    pub slot: u64,
    pub price: u64,
}

#[event(borsh)]
pub struct MetadataUpdated {
    pub uri: String,
    pub tags: Vec<[u8; 32]>,
    pub maybe_count: Option<u64>,
}

// emit!(Deposited { depositor, amount });
```

## Error handling

`require!`-family macros desugar to `return Err(err.into())`:

```rust
require!(cond, MyError::Unauthorized);
require_eq!(a, b);
require_neq!(a, b);
require_keys_eq!(addr_a, addr_b);
require_keys_neq!(addr_a, addr_b);
require_gt!(val, 0);
require_gte!(val, 1);
```

`Error` is a type alias for `ProgramError`. `#[error_code]` enums provide `From` into `Error`; raw `ProgramError::*` variants work too.

## CPI helpers

```rust
use anchor_lang_v2::prelude::*;

// Simple account creation (payer signs)
create_account(&payer, &new_account, space, &owner_program)?;

// PDA-signed creation — seed slice must include the bump as final element
create_account_signed(
    &payer,
    &new_account,
    space,
    &owner_program,
    &[b"vault", user.address().as_ref(), &[bump]],
)?;

// Static PDA derivation
let (address, bump) = find_program_address(
    &[b"vault", user.address().as_ref()],
    program_id,
);

// Verify an untrusted bump is canonical (includes curve check)
let bump = verify_program_address(&[b"vault", ...], program_id, &expected_addr)?;

// Skip curve check when the account already has data
let bump = find_and_verify_program_address_skip_curve(
    &[b"vault", ...], program_id, &expected_addr,
)?;
```

## Patterns

### Alignment-1 Pod wrappers

Raw `u128` / `i128` in pod accounts can trip the no-padding check: on x86_64 hosts they're align-16, on SBF they're align-8. A struct that's perfectly packed on SBF can therefore fail the cross-target assertion. The `pod` module provides alignment-1 wrappers — `PodU64`, `PodU32`, `PodU16`, `PodU128`, `PodI*`, `PodBool`, `PodVec<T, N>` — that guarantee tight packing regardless of the native type's alignment. Reach for `PodU128` / `PodI128` when you need 128-bit fields, and for the narrower wrappers when a struct's overall layout needs alignment-1 anywhere it's embedded.

### Bitmap state tracking

```rust
pub struct Vote {
    pub approval_bitmap: PodU16,
}

impl Vote {
    pub fn set_approval(&mut self, idx: u8) {
        self.approval_bitmap |= PodU16::from(1u16 << idx);
    }
    pub fn has_approved(&self, idx: u8) -> bool {
        self.approval_bitmap.get() & (1 << idx) != 0
    }
    pub fn approval_count(&self) -> u8 {
        self.approval_bitmap.get().count_ones() as u8
    }
}
```

### PDA with stored bump

Store the bump when your program later signs CPIs with this PDA. If every instruction re-validates via `#[account(seeds = [...], bump)]`, storing it is optimization, not correctness.

```rust
#[account]
pub struct Wallet {
    pub authority: Address,
    pub bump: u8,
}

pub fn create_wallet(ctx: &mut Context<CreateWallet>) -> Result<()> {
    ctx.accounts.wallet.authority = *ctx.accounts.payer.address();
    ctx.accounts.wallet.bump = ctx.bumps.wallet;
    Ok(())
}
```

### `remaining_accounts()` is lazy-cached

First call walks the cursor; subsequent calls clone the cached vec. Always validate count and owner before CPI dispatch.

```rust
pub fn execute(ctx: &mut Context<Execute>) -> Result<()> {
    let remaining = ctx.remaining_accounts();
    require!(remaining.len() % 2 == 0, ErrorCode::ConstraintRaw);
    for chunk in remaining.chunks(2) {
        let (from, to) = (&chunk[0], &chunk[1]);
        require_keys_eq!(*from.owner(), Token::id());
        require_keys_eq!(*to.owner(), Token::id());
        // ... CPI
    }
    Ok(())
}
```

### Validation order

Declarative constraints run in field order; your handler body runs after. Inside handlers, order by security criticality, not by CU cost: signature/authorization checks, then membership/has-one, then state-machine, then timelock, then mutation.

## Feature flags

```toml
[dependencies]
anchor-lang-v2 = { git = "...", branch = "anchor-next" }
```

| Feature | Default | Purpose |
|---|---|---|
| `alloc` | ✅ | Enables `pinocchio/alloc` — installs the on-chain bump allocator. Required for on-chain code that uses `Vec`/`String`/`Box`; host-side tests always have `std`'s allocator. |
| `guardrails` | ✅ | Runtime safety: writable check, use-after-release panic, executable check. ~0–5 CU per instruction. |
| `account-resize` | ✅ | Enables `realloc_account()`. Compile error if a program calls `realloc_account` without this feature. |
| `const-rent` | ❌ | Bakes rent formula into the binary. Saves ~85 CU per `create_account`; locks you into redeploying when the runtime rent formula changes. Enable only for high-creation programs. |
| `idl-build` | ❌ | Internal flag the `anchor idl build` pipeline sets. Don't enable manually. |
| `compat` | ❌ | v1-compat shims. Currently adds a `debug!` macro that accepts any `format!` pattern (`{:?}`, `{:x}`, dynamic width) via `alloc::format!`. Default-off because the heap-alloc + trait-dispatch costs are higher than `msg!`. |

Feature flags the **scaffold** emits in your program's `Cargo.toml` (`cpi`, `no-entrypoint`, `no-log-ix-name`, `profile`) are program-level features, not `anchor-lang-v2` features.

## Status

**Working today:**
- Account types: `Account<T>`, `BorshAccount<T>`, `Signer`, `Program<T>`, `SystemAccount`, `UncheckedAccount`, `Sysvar<T>`, `Slab<H, Item>`, `Option<Account<T>>`
- Constraints: `init`, `init_if_needed`, `mut`, `has_one`, `seeds`, `bump`, `constraint`, `close`, `owner`, `address`, `rent_exempt`, `token::*`, `mint::*`, `associated_token::*`
- Macros: `#[program]`, `#[derive(Accounts)]`, `#[account]` / `#[account(borsh)]`, `#[event]` / `#[event(bytemuck)]` / `#[event(borsh)]`, `#[error_code]`, `#[constant]`, `#[access_control]`, `#[derive(InitSpace)]`
- CPIs: system program create, PDA-signed, remaining-accounts injection
- SPL: `Mint`, `TokenAccount`, associated-token init, full CPI surface (`Transfer`, `MintTo`, `Burn`, `Approve`, `Revoke`, `SetAuthority`, `CloseAccount`, `Freeze`/`Thaw`, `SyncNative` + checked variants) via `spl-v2`
- In-process testing via LiteSVM + `anchor-v2-testing`, flamegraphs via `anchor test --profile`

**Known gaps:**

- **TypeScript client partially tested.** `anchor idl build` now emits `events` metadata, but the `@anchor-lang/core` `EventParser` and `AccountsResolver` paths haven't been validated against v2 IDLs end-to-end. Basic RPC calls and account decoding work; complex client-side decodes may need manual work.
- **Token2022 not supported.** `Mint`/`TokenAccount` validate against SPL Token only; Token2022 programs fail owner checks silently. Other SPL programs (governance, name-service, stake-pool, etc.) aren't wrapped.
- **Not on crates.io.** Must depend via git on the `anchor-next` branch until publish. Expected to stay that way through pre-1.0.

## v1 → v2

| Aspect | v1 | v2 |
|---|---|---|
| Runtime | `solana_program` | `pinocchio` |
| Stdlib | `std` required | `#![no_std]`, `alloc` is a feature |
| Binary size | baseline | −81% to −94% (see intro benchmarks) |
| Lifetime on Accounts | `<'info>` required everywhere | removed |
| Context in handlers | `Context<T>` | `&mut Context<T>` |
| Field access | `.key()` | `.address()` |
| `Account<T>` default | borsh-based | pod-based (v1 behavior → `BorshAccount<T>`) |
| `init` space | `space = 8 + T::INIT_SPACE` required | optional for pod `Account<T>` (defaults to `8 + size_of::<T>()`) |
| Pubkey type | `Pubkey` | `Address` (current SDK aliases `Pubkey` to `Address`, so both resolve to the same struct) |
| Dispatch | macro-generated match | trait-based direct calls |

## Examples

Worked examples live under [`bench/programs/`](../bench/programs/). The anchor-v2 variants are what the prototype benchmarks measure; the same directory also holds v1/quasar/steel ports for direct comparison.

| Program | Description |
|---|---|
| [`helloworld`](../bench/programs/helloworld/anchor-v2) | Single-instruction counter. Minimum viable `#[program]` + `#[account]`. |
| [`vault`](../bench/programs/vault/anchor-v2) | Single-depositor SOL vault. Two handlers (`deposit`, `withdraw`), PDA with stored bump, signed CPI. |
| [`multisig`](../bench/programs/multisig/anchor-v2) | Four-instruction SOL multisig. Multi-account state, PDA-signed transfers, `has_one` / `constraint` patterns. |
| [`prop-amm`](../bench/programs/prop-amm/anchor-v2) | Oracle price-feed with asm fast-path (`update` = 26 CU). Custom entrypoint + normal anchor handlers. |

## Tooling

### `anchor test --profile`

Flamegraph generation. Rebuilds with DWARF symbols, runs the test suite via LiteSVM in-process (no validator), captures per-test SBF register traces, and renders one SVG flamegraph per transaction under `target/anchor-v2-profile/`.

```bash
anchor test --profile
# prints paths: target/anchor-v2-profile/<test>__tx<N>.svg
```

The SVGs are written to disk and their paths are printed — open them in a browser manually. The scaffold's `Cargo.toml` includes a `profile` feature that forwards to `anchor-v2-testing/profile` — the register-tracing callback that captures the traces. `anchor test --profile` enables this feature automatically.

### `anchor debugger`

Foundry-style instruction-level TUI debugger. Reuses the `--profile` trace pipeline — builds with DWARF, runs tests, then opens a `ratatui` terminal UI over the captured register traces instead of rendering flamegraphs. Source-mapped: shows Rust source alongside the SBF disassembly with per-instruction CU.

```bash
anchor debugger                      # build + test + open TUI
anchor debugger my_test_name         # filter to tests matching "my_test_name"
anchor debugger --skip-run           # open TUI over existing traces
```

Both tools run entirely in-process via LiteSVM — no validator process, no network.

## Architecture

```
my_program/
├── src/
│   ├── lib.rs           // declare_id! + #[program]
│   ├── state.rs         // #[account] / #[account(borsh)] types
│   ├── error.rs         // #[error_code]
│   ├── constants.rs     // #[constant]
│   ├── instructions/
│   │   ├── mod.rs
│   │   └── initialize.rs
│   └── events.rs        // #[event] (optional)
└── Cargo.toml
```

## Key code paths

| Concept | File |
|---|---|
| Account types | `src/accounts/mod.rs` |
| Core traits (`AnchorAccount`, `Owner`, `Discriminator`, `Constrain`) | `src/traits.rs` |
| `Account<T>` / `Slab<H, Item>` — borrow tracking, exit semantics | `src/accounts/slab.rs` |
| `BorshAccount<T>` | `src/accounts/borsh_account.rs` |
| Derive macros | `derive/src/lib.rs` |
| Constraint parsing | `derive/src/parse.rs` |
| Dispatch + `Context` setup | `src/dispatch.rs` |
| CPI helpers + PDA derivation | `src/cpi.rs` |
| Pod alignment-1 wrappers | `src/pod.rs` |
| Program ID markers (`System`, `Token`, `Token2022`, `AssociatedToken`, `Memo`) | `src/programs.rs` |
| SPL wrappers | `../spl-v2/src/` |

## Contributing

File issues at [solana-foundation/anchor](https://github.com/solana-foundation/anchor/issues), tagged with `v2` where applicable. Working branch: `anchor-next`. See the top-level [CONTRIBUTING.md](../CONTRIBUTING.md).
