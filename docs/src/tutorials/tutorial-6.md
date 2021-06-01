# Associated Accounts

No Solana program should be written without understanding associated accounts.
Every program using an account in a way that maps to a user, which is almost all
programs, should use them.

The TLDR. Associated accounts are accounts whose address are deterministically defined by
a program and some associated data. Usually that data is both a user wallet and some account
instance, for example, a token mint.

Why should you care? UX.

Consider a wallet. Would you rather have a wallet with a single SOL address, which you
can use to receive *all* SPL tokens, or would you rather have a wallet with a different
address for every SPL token? Now generalize this. For every program you use, do you
want a single account, i.e. your SOL wallet, to define your application state? Or do
you want to keep track of all your account addresses, separately, for every program in existance?

Associated accounts allow your users to reason about a single address, their main SOL wallet.  This is
a huge improvement on the account model introduced thus far.

Luckily, Anchor provides the ability to easily create associated program accounts for your program.

::: details
If you've explored Solana, you may have come across the [Associated Token Account Program](https://spl.solana.com/associated-token-account) which uniquely and deterministically defines
a token account for a given wallet and a given mint. That is, if you have a SOL address,
then you will have, at most, a single "token account" for every SPL mint in existence
if you only use associated token addresses.

Unfortunately, the SPL token program doesn't do this, strictly. It was built *before* the existance
of associated token accounts (associated token accounts were built as an add-on).
So in reality, there are non associated token accounts floating around Solanaland.
However, for new programs, this isn't necessary, and it's recommend to only use associated
accounts when creating accounts on behalf of users, such as a token account.
:::

## Clone the Repo

To get started, clone the repo.

```bash
git clone https://github.com/project-serum/anchor
```

And change directories to the [example](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-5).

```bash
cd anchor/examples/tutorial/basic-5
```

## Defining a Program to Create Associated Accounts

The following program is an *extremely* simplified version of the SPL token program that
does nothing other than create a mint and *associated* token account.

<<< @/../examples/tutorial/basic-5/programs/basic-5/src/lib.rs#code

### Deriving Accounts

Two new keywords are introduced to the `CreateToken` account context:

* `associated = <target>`
* `with = <target>`

Both of these allow you to define input "seeds" that
uniquely define the associated account. By convention, `associated` is used to define
the main address to associate, i.e., the wallet, while `with` is used to define an
auxilliary piece of metadata which has the effect of namespacing the associated account.
This can be used, for example, to create multiple different associated accounts, each of
which is associated *with* a new piece of metadata. In the token program, these pieces
of metadata are mints, i.e., different token types.

Lastly, notice the two accounts at the bottom of account context.

```rust
    rent: Sysvar<'info, Rent>,
    system_program: AccountInfo<'info>,
```

In the same way that `rent` is required when using `init` in the previous tutorials,
`rent` and additionally the `system-program` must be provided when creating an associated
account. By convention, the names must be as given here.

For more details on how to use `#[account(associated)]`, see [docs.rs](https://docs.rs/anchor-lang/latest/anchor_lang/derive.Accounts.html).

### Defining an Associated Account

The new `#[acount(associated)]` attribute will allow you to create a new associated account similar to `#[account(init)]`, but
to actually define an account as associated, you must use the `#[associated]` attribute *instead* of the `#[account]` attribute.

This new `#[associated]` attribute extends `#[account]` to include two things

* A `Default` implementation, which is required for automatic size detection (performed when `#[account(space = "<size>")]` is omitted from the accounts context).
* The implementation of the [Bump](https://docs.rs/anchor-lang/latest/anchor_lang/trait.Bump.html) trait, which is a bit of an implementation but is required for program derived addresses on Solana.

## Using the Client

The client can be used similarly to all other examples. Additionally, we introduce
two new apis.

* `<program>.account.<account-name>.associatedAddress` returns an associated token address, given seeds.
* `<program>.account.<account-name>.associated` returns the deserialized associated account, given seeds.


We can use them with the example above as follows

<<< @/../examples/tutorial/basic-5/tests/basic-5.js#test

Notice that, in both apis, the "seeds" given match what is expected by the `#[account(associated = <target, with = <target>)]` attribute, where order matters. The `associated` target must come before the `with` target.

## Conclusion

Here, we introduced associated accounts from the perspective of simplifying UX for
a user wallet. However, deterministic addressing can be used beyond this and is a convenient
tool to have in your Solana toolbox. For more, it's recommended to see the Solana [docs](https://docs.solana.com/developing/programming-model/calling-between-programs#program-derived-addresses).
