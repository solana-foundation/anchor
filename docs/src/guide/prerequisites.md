# Prerequisites

Before getting started, make sure to setup all the prerequisite tools on your local machine.

## Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup component add rustfmt
```

For an introduction to Rust, see the excellent Rust [book](https://doc.rust-lang.org/book/).

## Install Anchor

For now, we can use Cargo.

```bash
cargo install --git https://github.com/project-serum/anchor anchor-cli
```

## Install Solana

```bash
curl -sSf https://raw.githubusercontent.com/solana-labs/solana/v1.4.14/install/solana-install-init.sh | sh -s - v1.4.14
export PATH="/home/ubuntu/.local/share/solana/install/active_release/bin:$PATH"
```

## Setup a Localnet

The easiest way to run a local cluster is to run the docker container provided by Solana. Instructions can be found [here](https://solana-labs.github.io/solana-web3.js/). (Note: `solana-test-validator` is the new, preferred way to run a local validator, though I haven't tested it yet).
