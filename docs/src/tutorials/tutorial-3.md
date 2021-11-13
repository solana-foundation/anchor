# Cross Program Invocations (CPI)

This tutorial covers how to call one program from another, a process known as
*cross-program-invocation* (CPI).

## Clone the Repo

To get started, clone the repo.

```bash
git clone https://github.com/project-serum/anchor
```

Change directories to the [example](https://github.com/project-serum/anchor/tree/master/examples/tutorial/basic-3).

```bash
cd anchor/examples/tutorial/basic-3
```

And install any additional JavaScript dependencies:

```bash
yarn install
```

## Defining a Puppet Program

We start with the program that will be called by another program, the puppet.

<<< @/../examples/tutorial/basic-3/programs/puppet/src/lib.rs

If you've followed along the other tutorials, this should be straight forward. We have
a program with two instructions, `initialize`, which does nothing other than the
initialization of the account (remember, the program *transparently* prepends a unique 8
byte discriminator the first time an account is used), and `set_data`, which takes a previously
initialized account, and sets its data field.

Now, suppose we wanted to call `set_data` from another program.

## Defining a Puppet Master Program

We define a new `puppet-master` crate, which successfully executes the Puppet program's `set_data`
instruction via CPI.

<<< @/../examples/tutorial/basic-3/programs/puppet-master/src/lib.rs#core

Things to notice

* We create a `CpiContext` object with the target instruction's accounts and program,
  here `SetData` and `puppet_program`.
* To invoke an instruction on another program, just use the `cpi` module on the crate, here, `puppet::cpi::set_data`.
* Our `Accounts` struct contains the puppet account we are calling into via CPI. Accounts used for CPI are not specifically denoted
  as such with the `CpiAccount` label since v0.15. Accounts used for CPI are not fundamentally different from `Program` or `Signer`
  accounts except for their role and ownership in the specific context in which they are used.

::: tip
When using another Anchor program for CPI, make sure to specify the `cpi` feature in your `Cargo.toml`.
If you look at the `Cargo.toml` for this example, you'll see
`puppet = { path = "../puppet", features = ["cpi"] }`.
:::

## Signer Seeds

Often it's useful for a program to sign instructions. For example, if a program controls a token
account and wants to send tokens to another account, it must sign. In Solana, this is done by specifying
"signer seeds" on CPI. To do this using the example above, simply change
`CpiContext::new(cpi_accounts, cpi_program)` to
`CpiContext::new_with_signer(cpi_accounts, cpi_program, signer_seeds)`.

For more background on signing with program derived addresses, see the official Solana [documentation](https://docs.solana.com/developing/programming-model/calling-between-programs#program-signed-accounts).

## Return values

Solana currently has no way to return values from CPI, alas. However, you can approximate this
by having the callee write return values to an account and the caller read that account to
retrieve the return value. In future work, Anchor should do this transparently.

## Conclusion

Now that you can have your programs call other programs, you should be able to access all the work being done by other developers in your own applications!

## Next Steps

We just covered Cross Program Invocation and showed how anchor can handle talking to multiple different programs in the solana ecosystem. In the next step, we will teach you how to handle errors and in Anchor.