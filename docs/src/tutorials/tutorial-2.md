# Account Constraints and Access Control

This tutorial covers how to specify constraints and access control on accounts, a problem
somewhat unique to the parallel nature of Solana.

On Solana, a transaction must specify all accounts required for execution. And because an untrusted client specifies those accounts, a program must responsibly validate all such accounts are what the client claims they are--in addition to any instruction specific access control the program needs to do.

For example, you could imagine easily writing a faulty token program that forgets to check if the **signer** of a transaction claiming to be the **owner** of a Token `Account` actually matches the **owner** on that account. Furthermore, imagine what might happen if the program expects a `Mint` account but a malicious user gives a token `Account`.

To address these problems, Anchor provides several types, traits, and macros. It's easiest to understand by seeing how they're used in an example, but a couple include

- [Accounts](https://docs.rs/anchor-lang/latest/anchor_lang/derive.Accounts.html): derive macro implementing the `Accounts` [trait](https://docs.rs/anchor-lang/latest/anchor_lang/trait.Accounts.html), allowing a struct to transform
  from the untrusted `&[AccountInfo]` slice given to a Solana program into a validated struct
  of deserialized account types.
- [#[account]](https://docs.rs/anchor-lang/latest/anchor_lang/attr.account.html): attribute macro implementing [AccountSerialize](https://docs.rs/anchor-lang/latest/anchor_lang/trait.AccountSerialize.html) and [AccountDeserialize](https://docs.rs/anchor-lang/latest/anchor_lang/trait.AnchorDeserialize.html), automatically prepending a unique 8 byte discriminator to the account array. The discriminator is defined by the first 8 bytes of the `Sha256` hash of the account's Rust identifier--i.e., the struct type name--and ensures no account can be substituted for another.
- [Account](https://docs.rs/anchor-lang/latest/anchor_lang/struct.Account.html): a wrapper type for a deserialized account implementing `AccountDeserialize`. Using this type within an `Accounts` struct will ensure the account is **owned** by the address defined by `declare_id!` where the inner account was defined.

With the above, we can define preconditions for any instruction handler expecting a certain set of
accounts, allowing us to more easily reason about the security of our programs.

## Clone the Repo

To get started, clone the repo.

```bash
git clone https://github.com/project-serum/anchor
```

Change directories to the [example](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-2).

```bash
cd anchor/examples/tutorial/basic-2
```

And install any additional JavaScript dependencies:

```bash
yarn install
```

## Defining a Program

Here we have a simple **Counter** program, where anyone can create a counter, but only the assigned
**authority** can increment it.

<<< @/../examples/tutorial/basic-2/programs/basic-2/src/lib.rs

If you've gone through the previous tutorials the `create` instruction should be straightforward.
Let's focus on the `increment` instruction, specifically the `Increment` struct deriving
`Accounts`.

```rust
#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}
```

Here, a couple `#[account(..)]` attributes are used.

- `mut`: tells the program to persist all changes to the account.
- `has_one`: enforces the constraint that `Increment.counter.authority == Increment.authority.key`.

Another new concept here is the `Signer` type. This enforces the constraint that the `authority`
account **signed** the transaction. However, anchor doesn't fetch the data on that account.

If any of these constraints do not hold, then the `increment` instruction will never be executed.
This allows us to completely separate account validation from our program's business logic, allowing us
to reason about each concern more easily. For more, see the full [list](https://docs.rs/anchor-lang/latest/anchor_lang/derive.Accounts.html) of account constraints.

## Next Steps

We've covered the basics for writing a single program using Anchor on Solana. But the power of
blockchains come not from a single program, but from combining multiple _composable_ programs
(buzzword...check). Next, we'll see how to call one program from another.
