# State structs

Up until now, we've treated programs on Solana as stateless, using accounts to persist
state between instruction invocations. In this tutorial, we'll give Solana programs the
illusion of state by introducing state structs, which define program account
singletons that can be operated over like any other account.

## Clone the Repo

To get started, clone the repo.

```bash
git clone https://github.com/project-serum/anchor
```

And change directories to the [example](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-4).

```bash
cd anchor/examples/tutorial/basic-4
```

## Defining a Program

<<< @/../examples/tutorial/basic-4/programs/basic-4/src/lib.rs#code

Unlike the previous examples, all the instructions here not only take in an `Accounts`
struct, but they also operate over a mutable, global account marked by the `#[state]`
attribute. Every instruction defined in the corresponding `impl` block will have access
to this account, making it a great place to store global program state.

### How it works

We are able to give a program the illusion of state by adopting conventions in the framework.  When invoking the `new` constructor, Anchor will automatically create a
program-owned account inside the program itself, invoking the system program's [create_account_with_seed](https://docs.rs/solana-program/1.5.5/solana_program/system_instruction/fn.create_account_with_seed.html) instruction, using `Pubkey::find_program_address(&[], program_id)` as the **base** and a deterministic string as the **seed** (the string doesn't
matter, as long as the framework is consistent).

This all has the effect of
giving the `#[state]` account a deterministic address, and so as long as all clients
and programs adopt this convention, programs can have the illusion of state in addition
to the full power of the lower level Solana accounts API. Of course, Anchor will handle this all for you, so you never have to worry about these details.

## Using the client

### Invoke the constructor

To access the `#[state]` account and associated instructions, you can use the
`anchor.state` namespace on the client. For example, to invoke the constructor,

<<< @/../examples/tutorial/basic-4/tests/basic-4.js#ctor

Note that the constructor can only be invoked once per program. All subsequent calls
to it will fail, since, as explained above, an account at a deterministic address
will be created.

### Fetch the state

To fetch the state account,

<<< @/../examples/tutorial/basic-4/tests/basic-4.js#accessor

### Invoke an instruction

To invoke an instruction,

<<< @/../examples/tutorial/basic-4/tests/basic-4.js#instruction

## CPI

Performing CPI from one Anchor program to another's state methods is very similar to performing CPI to normal Anchor instructions, except for two differences:

1. All the generated instructions are located under the `<my_program>::cpi::state` module.
2. You must use a [CpiStateContext](https://docs.rs/anchor-lang/latest/anchor_lang/struct.CpiStateContext.html), instead of a `[CpiContext](https://docs.rs/anchor-lang/latest/anchor_lang/struct.CpiContext.html).

For a full example, see the `test_state_cpi` instruction, [here](https://github.com/project-serum/anchor/blob/master/examples/misc/programs/misc/src/lib.rs#L39).

## Conclusion

Using state structs is intuitive. However, due to the fact that accounts
on Solana have a fixed size, applications often need to use accounts
directly in addition to `#[state]` stucts.

## Next Steps

Next we'll discuss errors.
