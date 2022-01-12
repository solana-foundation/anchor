# Installation
## Rust

Go [here](https://www.rust-lang.org/tools/install) to install Rust.

## Solana

Go [here](https://docs.solana.com/cli/install-solana-cli-tools) to install Solana.

## Yarn

Go [here](https://yarnpkg.com/getting-started/install) to install Yarn.

## Anchor

### Install using pre-build binary on x86_64 Linux

Anchor binaries are available via an NPM package [`@project-serum/anchor-cli`](https://www.npmjs.com/package/@project-serum/anchor-cli). Only `x86_64` Linux is supported currently, you must build from source for other OS'.

### Build from source for other operating systems

For now, we can use Cargo to install the CLI.

```
cargo install --git https://github.com/project-serum/anchor --tag v0.20.1 anchor-cli --locked
```

On Linux systems you may need to install additional dependencies if cargo install fails. On Ubuntu,

```
sudo apt-get update && sudo apt-get upgrade && sudo apt-get install -y pkg-config build-essential libudev-dev
```

Now verify the CLI is installed properly.

```
anchor --version
```