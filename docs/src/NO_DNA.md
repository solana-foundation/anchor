# NO_DNA — Agent-Friendly CLI Mode

Anchor supports the [`NO_DNA`](https://no-dna.org) standard for non-human operator
detection. When you run Anchor commands from an AI agent, CI pipeline, or any
non-interactive script, set `NO_DNA=1` to opt into agent-friendly behaviour.

## What it does

| Without `NO_DNA` | With `NO_DNA=1` |
|---|---|
| Interactive yes/no prompts block execution | Prompts are auto-confirmed (yes) and logged to stderr |
| Spinner / TUI output that cannot be parsed | Clean line-by-line stderr output prefixed with `[NO_DNA]` |
| Human-readable progress messages | Structured, machine-parseable progress |

## Usage

Prefix any `anchor` command with `NO_DNA=1`:

```bash
NO_DNA=1 anchor build
NO_DNA=1 anchor test
NO_DNA=1 anchor deploy
NO_DNA=1 anchor idl init --filepath target/idl/my_program.json my_program_id
```

Or export it for the duration of a session / CI job:

```bash
export NO_DNA=1
anchor build
anchor test
```

## How it works

The Anchor CLI checks the `NO_DNA` environment variable at startup via
`anchor_cli::no_dna::is_no_dna()`. Any truthy value (`1`, `true`, `yes`) activates
agent mode. All other values (including unset) use normal interactive behaviour.

## For contributors

When adding new interactive prompts to the CLI, always gate them through
`anchor_cli::no_dna::confirm(prompt)` instead of reading stdin directly. This
ensures agent compatibility without extra effort.

```rust
use anchor_cli::no_dna::confirm;

if confirm("Deploy to mainnet?")? {
    // proceed
}
```

Use the `no_dna_log!` macro to emit agent-visible progress messages:

```rust
use anchor_cli::no_dna_log;

no_dna_log!("Building program: {}", program_name);
```

## Standard

See [no-dna.org](https://no-dna.org) for the full cross-tool standard.
