# A Minimal Example

Here, we introduce Anchor's core syntax elements and project workflow. This tutorial assumes all
[prerequisites](../getting-started/installation.md) are installed.

## Clone the Repo

To get started, clone the repo.

```bash
git clone https://github.com/project-serum/anchor
```

Next, checkout the tagged branch of the same version of the anchor cli you have installed.

```bash
git checkout tags/<version>
```

Change directories to the [example](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-0).

```bash
cd anchor/examples/tutorial/basic-0
```

And install any additional JavaScript dependencies:

```bash
yarn install
```

## Starting a Localnet

In a separate terminal, start a local network. If you're running solana
for the first time, generate a wallet.

```
solana-keygen new
```

Then run

```
solana-test-validator
```

Then, shut it down.

The test validator will be used when testing Anchor programs. Make sure to turn off the validator before you begin testing Anchor programs.

::: details
As you'll see later, starting a localnet manually like this is not necessary when testing with Anchor,
but is done for educational purposes in this tutorial.
:::

## Defining a Program

We define the minimum viable program as follows.

<<< @/../examples/tutorial/basic-0/programs/basic-0/src/lib.rs

* `#[program]` First, notice that a program is defined with the `#[program]` attribute, where each
inner method defines an RPC request handler, or, in Solana parlance, an "instruction"
handler. These handlers are the entrypoints to your program that clients may invoke, as
we will see soon.

* `Context<Initialize>` The first parameter of _every_ RPC handler is the `Context` struct, which is a simple
container for the currently executing `program_id` generic over
`Accounts`--here, the `Initialize` struct.

* `#[derive(Accounts)]` The `Accounts` derive macro marks a struct containing all the accounts that must be
specified for a given instruction. To understand Accounts on Solana, see the
[docs](https://docs.solana.com/developing/programming-model/accounts).
In subsequent tutorials, we'll demonstrate how an `Accounts` struct can be used to
specify constraints on accounts given to your program. Since this example doesn't touch any
accounts, we skip this (important) detail.

## Building and Emitting an IDL

After creating a program, you can use the `anchor` CLI to build and emit an IDL, from which clients
can be generated.

```bash
anchor build
```

::: details
The `build` command is a convenience combining two steps.

1) `cargo build-bpf`
2) `anchor idl parse -f program/src/lib.rs -o target/idl/basic_0.json`.
:::

Once run, you should see your build artifacts, as usual, in your `target/` directory. Additionally,
a `target/idl/basic_0.json` file is created. Inspecting its contents you should see

```json
{
  "version": "0.1.0",
  "name": "basic_0",
  "instructions": [
    {
      "name": "initialize",
      "accounts": [],
      "args": []
    }
  ]
}
```

From this file a client can be generated. Note that this file is created by parsing the `src/lib.rs`
file in your program's crate.

::: tip
If you've developed on Ethereum, the IDL is analogous to the `abi.json`.
:::

The `build` command also generates a random new keypair in the `target/` directory (if there's not one already) whose public key will be the address of your program once deployed. You can obtain the address by running `anchor keys list`.
Make sure that the public key inside your `lib.rs` (the argument to `declare_id!`) file and your `Anchor.toml` matches the one returned by `anchor keys list`. Then run `build` again to include the `lib.rs` changes in the build.

## Deploying

Once built, we can deploy the program by running

```bash
anchor deploy
```

## Generating a Client

Now that we've built a program, deployed it to a local cluster, and generated an IDL,
we can use the IDL to generate a client to speak to our on-chain program. For example,
see [client.js](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-0/client.js).

<<< @/../examples/tutorial/basic-0/client.js#main

Notice how we dynamically created the `initialize` method under
the `rpc` namespace.

Now, make sure to plugin your program's address into `<YOUR-PROGRAM-ID>` (a mild
annoyance that we'll address next). In order to run the client, you'll also need the path
to your wallet's keypair you generated when you ran `solana-keygen new`; you can find it
by running

```bash
solana config get keypair
```

Once you've got it, run the client with the environment variable `ANCHOR_WALLET` set to
that path, e.g.

```bash
ANCHOR_WALLET=<YOUR-KEYPAIR-PATH> node client.js
```

You just successfully created a client and executed a transaction on your localnet.

## Workspaces

So far we've seen the basics of how to create, deploy, and make RPCs to a program, but
deploying a program, copy and pasting the address, and explicitly reading
an IDL is all a bit tedious, and can easily get out of hand the more tests and the more
programs you have. For this reason, we introduce the concept of a workspace.

Inspecting [tests/basic-0.js](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-0/tests/basic-0.js), we see the above example can be reduced to

<<< @/../examples/tutorial/basic-0/tests/basic-0.js#code

The `workspace` namespace provides access to all programs in the local project and is
automatically updated to reflect the latest deployment, making it easy to change
your program, update your JavaScript, and run your tests in a fast feedback loop.

::: tip NOTE
For now, the workspace feature is only available when running the `anchor test` command,
which will automatically `build`, `deploy`, and `test` all programs against a localnet
in one command.
:::

Finally, we can run the test. Don't forget to kill the local validator started earlier.
We won't need to start one manually for any future tutorials.

```bash
anchor test
```

## Next Steps

We've introduced the basic syntax of writing programs in Anchor along with a productive
workflow for building and testing. However, programs aren't all that interesting without
interacting with persistent state. We'll cover that next.
