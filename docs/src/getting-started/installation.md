# Installing Dependencies

To get started, make sure to setup all the prerequisite tools on your local machine
(an installer has not yet been developed).

## Install Rust

For an introduction to Rust, see the excellent Rust [book](https://doc.rust-lang.org/book/).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup component add rustfmt
```

## Install Solana

See the solana [docs](https://docs.solana.com/cli/install-solana-cli-tools) for installation instructions. On macOS and Linux,

```bash
sh -c "$(curl -sSfL https://release.solana.com/v1.9.1/install)"
```

## Install Yarn

[Yarn](https://yarnpkg.com/) is recommended for JavaScript package management.

```bash
npm install -g yarn
```

## Install Anchor

### Install using pre-build binary on x86_64 Linux

Anchor binaries are available via an NPM package [`@project-serum/anchor-cli`](https://www.npmjs.com/package/@project-serum/anchor-cli). Only x86_64 Linux is supported currently, you must build from source for other OS'.

```bash
npm i -g @project-serum/anchor-cli
```

### Build from source for other operating systems

For now, we can use Cargo to install the CLI.

```bash
cargo install --git https://github.com/project-serum/anchor --tag v0.24.2 anchor-cli --locked
```

On Linux systems you may need to install additional dependencies if `cargo install` fails. On Ubuntu,

```bash
sudo apt-get update && sudo apt-get upgrade && sudo apt-get install -y pkg-config build-essential libudev-dev
```

Now verify the CLI is installed properly.

```bash
anchor --version
```

## Start a Project

To initialize a new project, simply run:

```bash
anchor init <new-project-name>
```

## Minimum version requirements

| Build tool  | Version        |
|:------------|:---------------|
| Node.js     | v11.0.0        |
