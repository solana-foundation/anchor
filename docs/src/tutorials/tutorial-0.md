# Tutorial 0: A Minimal Example

Here, we introduce a minimal example demonstrating the Anchor workflow and core syntax
elements. This tutorial assumes all [prerequisites](./prerequisites.md) are installed and
a local network is running.

## Clone the Repo

To get started, clone the repo.

```bash
git clone https://github.com/project-serum/anchor
```

And change directories to the [example](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-0).

```bash
cd anchor/examples/tutorial/basic-0
```

## Defining a Program

We define the minimum viable program as follows.

<<< @/../examples/tutorial/basic-0/program/src/lib.rs

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

After creating a program, one can use the `anchor` CLI to build and emit an IDL, from which clients
can be generated.

```bash
anchor build
```

::: details
The `build` command is a convenience combining two steps.

1) `cargo build-bpf`
2) `anchor idl -f src/lib.rs -o basic.json`.
:::

Once run, you should see your build artifacts, as usual, in your `target/` directory. Additionally,
a `basic.json` file is created. Inspecting its contents you should see

```json
{
  "version": "0.0.0",
  "name": "basic",
  "instructions": [
    {
      "name": "initialize",
      "accounts": [],
      "args": []
    }
  ]
}
```

From which a client can be generated. Note that this file is created by parsing the `src/lib.rs`
file in your program's crate.

::: tip
If you've developed on Ethereum, the IDL is analogous to the `abi.json`.
:::

## Deploying a program

Once built, we can deploy the program using the `solana deploy` command.

```bash
solana deploy <path-to-your-repo>/anchor/target/deploy/basic_program_0.so
```

Making sure to susbstitute paths to match your local filesystem. Now, save the address
the program was deployed with. It will be useful later.

## Generating a Client

Now that we have an IDL, we can use it to create a client.

<<< @/../examples/tutorial/basic-0/app/client.js#main

Notice how the program dynamically created the `initialize` method under
the `rpc` namespace.

## Next Steps

So far we've seen the basics of how to create, deploy, and make RPCs to a program on Solana
using Anchor. But a program isn't all that interesting without interacting with it's
peristent state, which is what we'll cover next.
