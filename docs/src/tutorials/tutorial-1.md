# Tutorial 1: Accounts, Arguments, and Types

This tutorial covers the basics of creating and mutating accounts using Anchor.
It's recommended to read [Tutorial 0](./tutorial-0.md) first, as this tutorial will
build on top of it.

## Clone the Repo

To get started, clone the repo.

```bash
git clone https://github.com/project-serum/anchor
```

And change directories to the [example](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-1).

```bash
cd anchor/examples/tutorial/basic-1
```

## Defining a Program

We define our program as follows

<<< @/../examples/tutorial/basic-1/programs/basic-1/src/lib.rs#program

Some new syntax elements are introduced here.

First, notice the `data` argument passed into the program. This argument and any other valid
Rust types can be passed to the instruction to define inputs to the program. If you'd like to
pass in your own type, then it must be defined in the same `src/lib.rs` file as the
`#[program]` module (so that the IDL can pick it up). Additionally,
notice how we take a mutable reference to `my_account` and assign the `data` to it. This leads us
the `Initialize` struct, deriving `Accounts`.

There are two things to notice about `Initialize`. First, the
`my_account` field is marked with the `#[account(mut)]` attribute. This means that any
changes to the field will be persisted upon exiting the program. Second, the field is of
type `ProgramAccount<'info, MyAccount>`, telling the program it *must* be **owned**
by the currently executing program and the deserialized data structure is `MyAccount`.

In a later tutorial we'll delve more deeply into deriving `Accounts`. For now, just know
one must mark their accounts `mut` if they want them to, well, mutate. ;)

## Creating and Initializing Accounts

For a moment, assume an account of type `MyAccount` was created on Solana, in which case,
we can invoke the above `initialize` instruction as follows.

<<< @/../examples/tutorial/basic-1/tests/basic-1.js#code-separated

The last element passed into the method is common amongst all dynamically generated
methods on the `rpc` namespace, containing several options for a transaction. Here,
we specify the `accounts` field, an object of all the addresses the transaction
needs to touch.

::: details
If you've developed on Solana before, you might notice two things 1) the ordering of the accounts doesn't
matter and 2) the `isWritable` and `isSigner`
options are not specified on the account anywhere. In both cases, the framework takes care
of these details for you, by reading the IDL.
:::

However it's common--and sometimes necessary for security purposes--to batch
instructions together. We can extend the example above to both create an account
and initialize it in one atomic transaction.

<<< @/../examples/tutorial/basic-1/tests/basic-1.js#code

Here, notice the **two** fields introduced: `signers` and `instructions`. `signers`
is an array of all `Account` objects to sign the transaction and `instructions` is an
array of all instructions to run **before** the explicitly specified program instruction,
which in this case is `initialize`. Because we are creating `myAccount`, it needs to
sign the transaction, as required by the Solana runtime.

::: details
In future work, we might want to add something like a *Builder* pattern for constructing
common transactions like creating and then initializing an account.
:::

As before, we can run the example tests.

```
anchor test
```

## Next Steps

We've covered all the basics of developing applications using Anchor. However, we've
left out one import aspect to ensure the security of our programs--validating input
and access control. We'll cover that next.
