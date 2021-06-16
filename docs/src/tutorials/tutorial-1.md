# Arguments and Accounts

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

<<< @/../examples/tutorial/basic-1/programs/basic-1/src/lib.rs

Some new syntax elements are introduced here.

### `initialize` instruction

First, let's start with the initialize instruction. Notice the `data` argument passed into the program. This argument and any other valid
Rust types can be passed to the instruction to define inputs to the program.

::: tip
If you'd like to pass in your own type as an input to an instruction handler, then it must be
defined in the same `src/lib.rs` file as the `#[program]` module, so that the IDL parser can
pick it up.
:::

Additionally,
notice how we take a mutable reference to `my_account` and assign the `data` to it. This leads us to
the `Initialize` struct, deriving `Accounts`. There are three things to notice about `Initialize`.

1. The `my_account` field is of type `ProgramAccount<'info, MyAccount>`, telling the program it *must*
be **owned** by the currently executing program, and the deserialized data structure is `MyAccount`.
2. The `my_account` field is marked with the `#[account(init)]` attribute. This should be used
in one situation: when a given `ProgramAccount` is newly created and is being used by the program
for the first time (and thus its data field is all zero). If `#[account(init)]` is not used
when account data is zero initialized, the transaction will be rejected.
3. The `Rent` **sysvar** is required for the rent exemption check, which the framework enforces
by default for any account marked with `#[account(init)]`. To be more explicit about the check,
one can specify `#[account(init, rent_exempt = enforce)]`. To skip this check, (and thus
allowing you to omit the `Rent` acccount), you can specify
`#[account(init, rent_exempt = skip)]` on the account being initialized (here, `my_account`).

::: details
All accounts created with Anchor are laid out as follows: `8-byte-discriminator || borsh
serialized data`. The 8-byte-discriminator is created from the first 8 bytes of the
`Sha256` hash of the account's type--using the example above, `sha256("MyAccount")[..8]`.

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

We can simplify this further.

<<< @/../examples/tutorial/basic-1/tests/basic-1.js#code-simplified

As before, we can run the example tests.

```
anchor test
```

## Next Steps

We've covered all the basics of developing applications using Anchor. However, we've
left out one import aspect to ensure the security of our programs--validating input
and access control. We'll cover that next.
