# Tutorial 2: Account Constraints and Access Control

This tutorial covers how to specify constraints and access control on accounts.

Because Solana programs are stateless, a transaction must specify accounts to be executed. And because an untrusted client specifies those accounts, a program must responsibily validate all input to the program to ensure it is what it claims to be--in addition to any instruction specific access control the program needs to do.

For example, one could imagine easily writing a faulty token program that forgets to check if the signer of a transaction claiming to be the owner of a token account actually matches the owner on the account. A simpler question that must be asked: what happens if the program expects a `Mint` account but a `Token` account is given instead?


Doing these checks is particularly burdensome when there are lots of dependencies between
accounts, leading to repetitive [boilerplate](https://github.com/project-serum/serum-dex/blob/master/registry/src/access_control.rs)
code for account validation along with the ability to easily shoot oneself in the foot.
Instead, one can use the Anchor DSL to do these checks by specifying **constraints** when deriving
`Accounts`. We briefly touched on the most basic (and important) type of account constraint in the
[previous tutorial](./tutorial-1.md), the account discriminator. Here, we demonstrate others.

## Clone the Repo

To get started, clone the repo.

```bash
git clone https://github.com/project-serum/anchor
```

And change directories to the [example](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-2).

```bash
cd anchor/examples/tutorial/basic-2
```

## Defining a Program

For now see the [source](https://github.com/project-serum/anchor/tree/master/examples/basic-2).

TODO

## Next Steps

We've covered the basics for writing a single program using Anchor on Solana. But the power of
blockchains come not from a single program, but from combining multiple *composable* programs
(buzzword alert!). Next, we'll see how to call one program from another.
