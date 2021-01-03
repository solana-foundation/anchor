# Tutorial 0: A Minimal Example

Here, we introduce a minimal example demonstrating the Anchor workflow and core syntax
elements. This tutorial assumes all [prerequisites](./prerequisites.md) are installed and
a local network is running.

## Clone the repo

To get started, clone the repo.

```bash
git clone https://github.com/armaniferrante/anchor
```

And change directories to the [example](https://github.com/armaniferrante/anchor/tree/master/examples/basic-0).

```bash
cd anchor/examples/tutorial/basic-0
```

## Defining a program

We define the minimum viable program as follows.

<<< @/../examples/tutorial/basic-0/program/src/lib.rs

There are a couple of syntax elements to point out here.

### `#[program]`

First, notice that a program is defined with the `#[program]` attribute, where each
inner method defines an RPC request handler, or, in Solana parlance, an "instruction"
handler. These handlers are the entrypoints to your program that clients may invoke, as
we will see soon.

### `Context<Initialize>`

The first parameter of *every* RPC handler is the `Context` struct, which is a simple
container for the currently executing `program_id`  generic over
`Accounts`--here, the `Initialize` struct.

### `#[derive(Accounts)]`

The `Accounts` derive macro marks a struct containing all the accounts that must be
specified for a given instruction. To understand Accounts on Solana, see the
[docs](https://docs.solana.com/developing/programming-model/accounts).
In subsequent tutorials, we'll demonstrate how an `Accounts` struct can be used to
specify constraints on accounts given to your program. Since this example doesn't touch any
accounts, we skip this (important) detail.

## Building a program

This program can be built in same way as any other Solana program.

```bash
cargo build-bpf
```

## Deploying a program

Similarly, we can deploy the program using the `solana deploy` command.

```bash
solana deploy <path-to-your-repo>/anchor/target/deploy/basic_program_0.so
```

Making sure to susbstitute paths to match your local filesystem. Now, save the address
the program was deployed with. It will be useful later.

## Emmiting an IDL

After creating a program, one can use the Anchor CLI to emit an IDL, from which clients
can be generated.

```bash
anchor idl -f src/lib.rs -o idl.js
```
Inspecting the contents of `idl.js` one should see

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
For experienced Ethereum developers, this is analogous to an `abi.json` file.

## Generating a Client

Now that we have an IDL, we can use it to create a client.

<<< @/../examples/tutorial/basic-0/app/client.js#main

Notice how the program dynamically created the `initialize` method under
the `rpc` namespace.

## Next Steps

So far we've seen the basics of how to create, deploy, and make RPCs to a program on Solana
using Anchor. But a program isn't all that interesting without interacting with it's
peristent state, which is what we'll cover next.
