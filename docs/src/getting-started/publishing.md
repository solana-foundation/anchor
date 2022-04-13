# Publishing Source

The Anchor Program Registry at [anchor.projectserum.com](https://anchor.projectserum.com)
hosts a catalog of verified programs on Solana both written with and without Anchor. It is recommended
that authors of smart contracts publish their source to promote best
practices for security and transparency.

::: tip note
The Anchor Program Registry is currently in alpha testing. For access to publishing
please ask on [Discord](https://discord.gg/rg5ZZPmmTm).
:::

## Getting Started

The process for publishing is mostly identical to `crates.io`.

* Signup for an account [here](https://anchor.projectserum.com/signup).
* Confirm your email by clicking the link sent to your address.
* Navigate to your Username -> Account Settings on the top navbar.
* Click "New Token" in the **API Access** section.
* Run `anchor login <token>` at the command line.

And you're ready to interact with the registry.

## Configuring a Build

Whether your program is written in Anchor or not, all source being published must
have an `Anchor.toml` to define the build.

An example `Anchor.toml` config looks as follows,

```toml
anchor_version = "0.24.2"

[workspace]
members = ["programs/multisig"]

[provider]
cluster = "mainnet"
wallet = "~/.config/solana/id.json"

[programs.mainnet]
multisig = "A9HAbnCwoD6f2NkZobKFf6buJoN9gUVVvX5PoUnDHS6u"

[programs.localnet]
multisig = "A9HAbnCwoD6f2NkZobKFf6buJoN9gUVVvX5PoUnDHS6u"
```

Here there are four sections.

1. `anchor_version` (optional) - sets the anchor docker image to use. By default, the builder will use the latest version of Anchor.
2. `[workspace]` (optional) - sets the paths--relative to the `Anchor.toml`--
   to all programs in the local
   workspace, i.e., the path to the `Cargo.toml` manifest associated with each
   program that can be compiled by the `anchor` CLI. For programs using the
   standard Anchor workflow, this can be ommitted.  For programs not written in Anchor
   but still want to publish, this should be added.
3. `[provider]` - configures the wallet and cluster settings. Here, `mainnet` is used because the registry only supports `mainnet` binary verification at the moment.
3. `[programs.mainnet]` - configures each program in the workpace, providing
   the `address` of the program to verify.

::: tip
When defining program in `[programs.mainnet]`, make sure the name provided
matches the **lib** name for your program, which is defined
by your program's Cargo.toml.
:::

### Examples

#### Anchor Program

An example of a toml file for an Anchor program can be found [here](https://anchor.projectserum.com/build/2).

#### Non Anchor Program

An example of a toml file for a non-anchor program can be found [here](https://anchor.projectserum.com/build/1).

## Publishing

To publish to the Anchor Program Registry, change directories to the `Anchor.toml`
defined root and run

```bash
anchor publish <program-name>
```

where `<program-name>` is as defined in `[programs.mainnet]`, i.e., `multisig`
in the example above.
