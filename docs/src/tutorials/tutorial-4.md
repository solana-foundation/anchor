# Tutorial 4: State structs

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

TODO: explain + add instructions (both manual instructions and instructions inside the impl block).

## Using the client

<<< @/../examples/tutorial/basic-4/tests/basic-4.js#code

TODO explain.
