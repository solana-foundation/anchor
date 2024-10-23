---
title: Installation
description: Anchor - Installation
---

## Rust

Go [here](https://www.rust-lang.org/tools/install) to install Rust.

{% callout title="You should know!" %}
We recommend reading chapters 1-9 of the [Rust book](https://doc.rust-lang.org/book/title-page.html) which cover the basics of using Rust (Most of the time you don't need advanced Rust to write anchor programs).
{% /callout %}

## Solana

Go [here](https://docs.solana.com/cli/install-solana-cli-tools) to install Solana and then run `solana-keygen new` to create a keypair at the default location. Anchor uses this keypair to run your program tests.

{% callout title="You should know!" %}
We also recommend checking out the official [Solana developers page](https://solana.com/developers).
{% /callout %}

## Yarn

Go [here](https://yarnpkg.com/getting-started/install) to install Yarn.

## Anchor

### Installing using Anchor version manager (avm) (recommended)

Anchor version manager is a tool for using multiple versions of the anchor-cli. It will require the same dependencies as building from source. It is recommended you uninstall the NPM package if you have it installed.

Install `avm` using Cargo. Note this will replace your `anchor` binary if you had one installed.

```shell
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
```

Install the latest version of the CLI using `avm`:

```shell
avm install latest
```

Verify the installation.

```shell
anchor --version
```

### Install using pre-build binary on x86_64 Linux

Anchor binaries are available via an NPM package [`@coral-xyz/anchor-cli`](https://www.npmjs.com/package/@coral-xyz/anchor-cli). Only `x86_64` Linux is supported currently, you must build from source for other OS'.

### Build from source for other operating systems without avm

We can also use Cargo to install the CLI directly. Make sure that the `--tag` argument uses the version you want (the version here is just an example).

```shell
cargo install --git https://github.com/coral-xyz/anchor --tag v0.30.1 anchor-cli --locked
```

Now verify the CLI is installed properly.

```shell
anchor --version
```

## Issues

Installation might fail due to a variety of reasons. This section contains a list of the most common issues and their solutions.

### Missing dependencies

On Linux systems you may need to install additional dependencies. E.g. on Ubuntu:

```shell
sudo apt-get update && sudo apt-get upgrade && sudo apt-get install -y pkg-config build-essential libudev-dev libssl-dev
```

### Incorrect `$PATH`

Rust binaries, including `avm` and `anchor`, are installed to the `~/.cargo/bin` directory. Since this directory is required to be in the `PATH` environment variable, [Rust](#rust) installation tries to set it up automatically, but it might fail to do so in some platforms.

To verify that the `PATH` environment variable was set up correctly, run:

```shell
which anchor
```

the output should look like (with your username):

```
/home/user/.cargo/bin/anchor
```

If the command fails or the output is empty, make sure to add the `~/.cargo/bin` directory to the `PATH` environment variable.

#### Bash

```shell
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

#### Fish

```shell
echo "set -gx PATH \$PATH \$HOME/.cargo/bin" >> ~/.config/fish/config.fish
source ~/.config/fish/config.fish
```

#### Zsh

```shell
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

## Generating Shell Completions

Shell completions can be generated for `bash`, `elvish`, `fish`, `powershell`, and `zsh`.

### Bash

```bash
mkdir -p $HOME/.local/share/bash-completion/completions
anchor completions bash > $HOME/.local/share/bash-completion/completions/anchor
avm completions bash > $HOME/.local/share/bash-completion/completions/avm
exec bash
```

### Fish

```bash
mkdir -p $HOME/.config/fish/completions
anchor completions fish > $HOME/.config/fish/completions/anchor.fish
avm completions fish > $HOME/.config/fish/completions/avm.fish
source $HOME/.config/fish/config.fish
```

### Zsh

First ensure the following is in your `.zshrc` file. If using `oh-my-zsh` this step can be skipped.

```bash
autoload -U compinit
compinit -i
```

Next run:

```bash
anchor completions zsh | sudo tee /usr/local/share/zsh/site-functions/_anchor
avm completions zsh | sudo tee /usr/local/share/zsh/site-functions/_avm
exec zsh
```
