# Quick Start

The quick start provides a whirlwind tour through creating, deploying, and testing a project
using Anchor, targeted at developers who are familiar with blockchain development. For an in depth
guide of Anchor from the ground up, see the subequent tutorials.

## Initialize a project

Anchor follows the principle of "Convention is better than configuration".
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

To build your program targeting Solana's BPF runtime and emit an IDL that can be
consumed by clients, run

```bash
anchor build
```

## Test

It's [recommended](https://www.parity.io/paritys-checklist-for-secure-smart-contract-development/)
to test your program using integration tests in a language other
than Rust to make sure that bugs related to syntax misunderstandings
are coverable with tests and not just replicated in tests.

```
anchor test
```

You just built a program, deployed it to a local network, and
ran integration tests in one command. It's that easy. ;)

## Deploy

To deploy all programs in your workspace, run

```
anchor deploy
```
