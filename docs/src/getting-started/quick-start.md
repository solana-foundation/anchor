# Quick Start

The quick start provides a whirlwind tour through creating, deploying, and testing a project
using Anchor. For an in depth guide building the Anchor workflow and DSL from the ground up,
see the subequent tutorials.

## Initialize a project

To initialize your project workspace, run

```bash
anchor init my-project && cd my-project
```

Your repo will be laid out with the following structure

* `Anchor.toml`: Anchor configuration file.
* `programs/`: Directory for Solana program crates.
* `app/`: Directory for your application frontend.
* `tests/`: Directory for JavaScript integration tests.

## Build

To build your program targeting Solana's BPF runtime and emit an IDL run

```bash
anchor build
```

You should see IDL files for all workspace programs in your `target/idl/` directory.

## Deploy

To deploy all programs in your workspace, run

```
anchor deploy
```

## Test

It's [recommended](https://www.parity.io/paritys-checklist-for-secure-smart-contract-development/)
to test your program using integration tests in a language other
than Rust to make sure that bugs related to syntax misunderstandings
are coverable with tests and not just replicated in tests.

```
anchor test
```

Testing will build a program, deploy it to a local network, and
run integration tests all in one command.

## New

Anchor supports simulatenous development of multiple Solana programs.
To create a new program in your workspace, run

```
anchor new <program-name>
```

You should see a new program in the `programs/` directory initialized with boilerplate.
