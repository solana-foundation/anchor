# Commands

A CLI is provided to support building and managing an Anchor workspace.
For a comprehensive list of commands and options, run `anchor -h` on any
of the following subcommands.

```
anchor-cli

USAGE:
    anchor <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    build      Builds the workspace
    cluster    Cluster commands
    deploy     Deploys each program in the workspace
    help       Prints this message or the help of the given subcommand(s)
    idl        Commands for interacting with interface definitions
    init       Initializes a workspace
    migrate    Runs the deploy migration script
    new        Creates a new program
    test       Runs integration tests against a localnetwork
    upgrade    Upgrades a single program. The configured wallet must be the upgrade authority
    verify     Verifies the on-chain bytecode matches the locally compiled artifact. Run this
               command inside a program subdirectory, i.e., in the dir containing the program's
               Cargo.toml
```

## Init

```
anchor init
```

Initializes a project workspace with the following structure.

* `Anchor.toml`: Anchor configuration file.
* `Cargo.toml`: Rust workspace configuration file.
* `package.json`: JavaScript dependencies file.
* `programs/`: Directory for Solana program crates.
* `app/`: Directory for your application frontend.
* `tests/`: Directory for JavaScript integration tests.
* `migrations/deploy.js`: Deploy script.

## Build

```
anchor build
```

Builds programs in the workspace targeting Solana's BPF runtime and emitting IDLs in the `target/idl` directory.

```
anchor build --verifiable
```

Runs the build inside a docker image so that the output binary is deterministic (assuming a Cargo.lock file is used). This command must be run from within a single crate subdirectory within the workspace. For example, `programs/<my-program>/`.

## Deploy

```
anchor deploy
```

Deploys all programs in the workspace to the configured cluster.

::: tip Note
This is different from the `solana program deploy` command, because everytime it's run
it will generate a *new* program address.
:::

## Upgrade

```
anchor upgrade <target/deploy/program.so> --program-id <program-id>
```

Uses Solana's upgradeable BPF loader to upgrade the on chain program code.

## Test

```
anchor test
```

Run an integration test suit against the configured cluster, deploying new versions
of all workspace programs before running them.

If the configured network is a localnet, then automatically starts the localnetwork and runs
the test.

::: tip Note
Be sure to shutdown any other local validators, otherwise `anchor test` will fail to run.

If you'd prefer to run the program against your local validator use `anchor test --skip-local-validator`.
:::

When running tests we stream program logs to `.anchor/program-logs/<address>.<program-name>.log`

::: tip Note
The Anchor workflow [recommends](https://www.parity.io/paritys-checklist-for-secure-smart-contract-development/)
to test your program using integration tests in a language other
than Rust to make sure that bugs related to syntax misunderstandings
are coverable with tests and not just replicated in tests.
:::

## Migrate

```
anchor migrate
```

Runs the deploy script located at `migrations/deploy.js`, injecting a provider configured
from the workspace's `Anchor.toml`. For example,

```javascript
// File: migrations/deploys.js

const anchor = require("@project-serum/anchor");

module.exports = async function (provider) {
  anchor.setProvider(provider);

  // Add your deploy script here.
}
```

Migrations are a new feature
and only support this simple deploy script at the moment.

## Idl

The `idl` subcommand provides commands for interacting with interface definition files.
It's recommended to use these commands to store an IDL on chain, at a deterministic
address, as a function of nothing but the the program's ID. This
allows us to generate clients for a program using nothing but the program ID.

### Idl Init

```
anchor idl init -f <target/idl/program.json> <program-id>
```

Creates an idl account, writing the given `<target/idl/program.json>` file into a program owned account. By default, the size of the account is double the size of the IDL,
allowing room for growth in case the idl needs to be upgraded in the future.

### Idl Fetch

```
anchor idl fetch -o <out-file.json> <program-id>
```

Fetches an IDL from the configured blockchain. For example, make sure
your `Anchor.toml` is pointing to the `mainnet` cluster and run

```
anchor idl fetch GrAkKfEpTKQuVHG2Y97Y2FF4i7y7Q5AHLK94JBy7Y5yv
```

### Idl Authority

```
anchor idl authority <program-id>
```

Outputs the IDL account's authority. This is the wallet that has the ability to
update the IDL.

### Idl Erase Authority

```
anchor idl erase-authority -p <program-id>
```

Erases the IDL account's authority so that upgrades can no longer occur. The
configured wallet must be the current authority.

### Idl Upgrade

```
anchor idl upgrade <program-id> -f <target/idl/program.json>
```

Upgrades the IDL file on chain to the new `target/idl/program.json` idl.
The configured wallet must be the current authority.

```
anchor idl set-authority -n <new-authority> -p <program-id>
```

Sets a new authority on the IDL account. Both the `new-authority` and `program-id`
must be encoded in base 58.

## New

```
anchor new <program-name>
```

Creates a new program in the workspace's `programs/` directory initialized with boilerplate.

## Cluster

### Cluster list

```
anchor cluster list
```

This lists cluster endpoints:

```
Cluster Endpoints:

* Mainnet - https://solana-api.projectserum.com
* Mainnet - https://api.mainnet-beta.solana.com
* Devnet  - https://api.devnet.solana.com
* Testnet - https://api.testnet.solana.com
```

## Verify

```
anchor verify <program-id>
```

Verifies the on-chain bytecode matches the locally compiled artifact.
