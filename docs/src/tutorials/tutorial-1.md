# Arguments and Accounts

This tutorial covers the basics of creating and mutating accounts using Anchor.
It's recommended to read [Tutorial 0](./tutorial-0.md) first, as this tutorial will
build on top of it.

## Clone the Repo

To get started, clone the repo.

```bash
git clone https://github.com/project-serum/anchor
```

Change directories to the [example](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-1).

```bash
cd anchor/examples/tutorial/basic-1
```

And install any additional JavaScript dependencies:

```bash
yarn install
```

## Defining a Program

We define our program as follows

<<< @/../examples/tutorial/basic-1/programs/basic-1/src/lib.rs

Some new syntax elements are introduced here.

### `initialize` instruction

First, let's start with the initialize instruction. Notice the `data` argument passed into the program. This argument and any other valid
Rust types can be passed to the instruction to define inputs to the program.

Additionally,
notice how we take a mutable reference to `my_account` and assign the `data` to it. This leads us to
the `Initialize` struct, deriving `Accounts`. There are two things to notice about `Initialize`.

1. The `my_account` field is of type `Account<'info, MyAccount>` and the deserialized data structure is `MyAccount`.
2. The `my_account` field is marked with the `init` attribute. This will create a new
account owned by the current program, zero initialized. When using `init`, one must also provide
`payer`, which will fund the account creation, `space`, which defines how large the account should be,
and the `system_program`, which is required by the runtime for creating the account.

::: details
All accounts created with Anchor are laid out as follows: `8-byte-discriminator || borsh
serialized data`. The 8-byte-discriminator is created from the first 8 bytes of the
`Sha256` hash of the account's type--using the example above, `sha256("account:MyAccount")[..8]`.
The `account:` is a fixed prefix.

Importantly, this allows a program to know for certain an account is indeed of a given type.
Without it, a program would be vulnerable to account injection attacks, where a malicious user
specifies an account of an unexpected type, causing the program to do unexpected things.

On account creation, this 8-byte discriminator doesn't exist, since the account storage is
zeroed. The first time an Anchor program mutates an account, this discriminator is prepended
to the account storage array and all subsequent accesses to the account (not decorated with
`#[account(init)]`) will check for this discriminator.
:::

### `update` instruction

Similarly, the `Update` accounts struct is marked  with the `#[account(mut)]` attribute.
Marking an account as `mut` persists any changes made upon exiting the program.

Here we've covered the basics of how to interact with accounts. In a later tutorial,
we'll delve more deeply into deriving `Accounts`, but for now, just know
you must mark an account `init` when using it for the first time and `mut`
for persisting changes.

## Creating and Initializing Accounts

We can interact with the program as follows.

<<< @/../examples/tutorial/basic-1/tests/basic-1.js#code-simplified

The last element passed into the method is common amongst all dynamically generated
methods on the `rpc` namespace, containing several options for a transaction. Here,
we specify the `accounts` field, an object of all the addresses the transaction
needs to touch, and the `signers` array of all `Signer` objects needed to sign the
transaction. Because `myAccount` is being created, the Solana runtime requries it
to sign the transaction.

::: details
If you've developed on Solana before, you might notice two things 1) the ordering of the accounts doesn't
matter and 2) the `isWritable` and `isSigner`
options are not specified on the account anywhere. In both cases, the framework takes care
of these details for you, by reading the IDL.
:::

As before, we can run the example tests.

```
anchor test
```

## Next Steps

We've covered all the basics of developing applications using Anchor. However, we've
left out one important aspect to ensure the security of our programs--validating input
and access control. We'll cover that next.
