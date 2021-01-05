# Tutorial 2: Account Constraints and Access Control

Building on the previous two, this tutorial covers how to speciy constraints and access control
on accounts. The full example can be found [here](https://github.com/project-serum/anchor/tree/master/examples/basic-2).

Because Solana programs are stateless, a transaction must specify accounts to be executed. And because an untrusted client specifies those accounts, a program must responsibily validate all input to the program to ensure it is what it claims to be--in addition to any instruction specific access control the program needs to do. This is particularly burdensome when there are lots of dependencies between accounts, leading to repetitive [boilerplate](https://github.com/project-serum/serum-dex/blob/master/registry/src/access_control.rs) code for account validation along with the ability to easily shoot oneself in the foot by forgetting to validate any particular account.

For example, one could imagine easily writing a faulty token program that forgets to check if the signer of a transaction claiming to be the owner of a token account actually matches the owner on the account. So one must write an `if` statement to check for all such conditions. Instead, one can use the Anchor DSL to do these checks by specifying **constraints** when deriving `Accounts`.

## Clone the Repo

To get started, clone the repo.

```bash
git clone https://github.com/project-serum/anchor
```

And change directories to the [example](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-1).

```bash
cd anchor/examples/tutorial/basic-2
```

## Defining a Program

For now see the [source](https://github.com/project-serum/anchor/tree/master/examples/basic-2).

TODO
