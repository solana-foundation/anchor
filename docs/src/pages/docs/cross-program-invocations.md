---
title: Cross-Program Invocations
description: Anchor - Cross-Program Invocations
---

Often it's useful for programs to interact with each other. In Solana this is achieved via Cross-Program Invocations (CPIs).

---

Consider the following example of a puppet and a puppet master. Admittedly, it is not very realistic but it allows us to show you the many nuances of CPIs. The milestone project of the intermediate section covers a more realistic program with multiple CPIs.

## Setting up basic CPI functionality

Create a new workspace

```shell
anchor init puppet
```

and copy the following code.

```rust
use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod puppet {
    use super::*;
    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn set_data(ctx: Context<SetData>, data: u64) -> Result<()> {
        let puppet = &mut ctx.accounts.puppet;
        puppet.data = data;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 8)]
    pub puppet: Account<'info, Data>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetData<'info> {
    #[account(mut)]
    pub puppet: Account<'info, Data>,
}

#[account]
pub struct Data {
    pub data: u64,
}
```

There's nothing special happening here. It's a pretty simple program! The interesting part is how it interacts with the next program we are going to create.

Run

```shell
anchor new puppet-master
```

inside the workspace and copy the following code:

```rust
use anchor_lang::prelude::*;
use puppet::cpi::accounts::SetData;
use puppet::program::Puppet;
use puppet::{self, Data};

declare_id!("HmbTLCmaGvZhKnn1Zfa1JVnp7vkMV4DYVxPLWBVoN65L");

#[program]
mod puppet_master {
    use super::*;
    pub fn pull_strings(ctx: Context<PullStrings>, data: u64) -> Result<()> {
        let cpi_program = ctx.accounts.puppet_program.to_account_info();
        let cpi_accounts = SetData {
            puppet: ctx.accounts.puppet.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        puppet::cpi::set_data(cpi_ctx, data)
    }
}

#[derive(Accounts)]
pub struct PullStrings<'info> {
    #[account(mut)]
    pub puppet: Account<'info, Data>,
    pub puppet_program: Program<'info, Puppet>,
}
```

Also add the line `puppet_master = "HmbTLCmaGvZhKnn1Zfa1JVnp7vkMV4DYVxPLWBVoN65L"` in the `[programs.localnet]` section of your `Anchor.toml`. Finally, import the puppet program into the puppet-master program by adding the following line to the `[dependencies]` section of the `Cargo.toml` file inside the `puppet-master` program folder:

```toml
puppet = { path = "../puppet", features = ["cpi"]}
```

The `features = ["cpi"]` is used so we can not only use puppet's types but also its instruction builders and cpi functions. Without those, we would have to use low level solana syscalls. Fortunately, anchor provides abstractions on top of those. By enabling the `cpi` feature, the puppet-master program gets access to the `puppet::cpi` module. Anchor generates this module automatically and it contains tailor-made instructions builders and cpi helpers for the program.

In the case of the puppet program, the puppet-master uses the `SetData` instruction builder struct provided by the `puppet::cpi::accounts` module to submit the accounts the `SetData` instruction of the puppet program expects. Then, the puppet-master creates a new cpi context and passes it to the `puppet::cpi::set_data` cpi function. This function has the exact same function as the `set_data` function in the puppet program with the exception that it expects a `CpiContext` instead of a `Context`.

Setting up a CPI can distract from the business logic of the program so it's recommended to move the CPI setup into the `impl` block of the instruction. The puppet-master program then looks like this:

```rust
use anchor_lang::prelude::*;
use puppet::cpi::accounts::SetData;
use puppet::program::Puppet;
use puppet::{self, Data};

declare_id!("HmbTLCmaGvZhKnn1Zfa1JVnp7vkMV4DYVxPLWBVoN65L");

#[program]
mod puppet_master {
    use super::*;
    pub fn pull_strings(ctx: Context<PullStrings>, data: u64) -> Result<()> {
        puppet::cpi::set_data(ctx.accounts.set_data_ctx(), data)
    }
}

#[derive(Accounts)]
pub struct PullStrings<'info> {
    #[account(mut)]
    pub puppet: Account<'info, Data>,
    pub puppet_program: Program<'info, Puppet>,
}

impl<'info> PullStrings<'info> {
    pub fn set_data_ctx(&self) -> CpiContext<'_, '_, '_, 'info, SetData<'info>> {
        let cpi_program = self.puppet_program.to_account_info();
        let cpi_accounts = SetData {
            puppet: self.puppet.to_account_info()
        };
        CpiContext::new(cpi_program, cpi_accounts)
    }
}
```

We can verify that everything works as expected by replacing the contents of the `puppet.ts` file with:

```ts
import * as anchor from '@project-serum/anchor'
import { Program } from '@project-serum/anchor'
import { Keypair } from '@solana/web3.js'
import { expect } from 'chai'
import { Puppet } from '../target/types/puppet'
import { PuppetMaster } from '../target/types/puppet_master'

describe('puppet', () => {
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider)

  const puppetProgram = anchor.workspace.Puppet as Program<Puppet>
  const puppetMasterProgram = anchor.workspace
    .PuppetMaster as Program<PuppetMaster>

  const puppetKeypair = Keypair.generate()

  it('Does CPI!', async () => {
    await puppetProgram.methods
      .initialize()
      .accounts({
        puppet: puppetKeypair.publicKey,
        user: provider.wallet.publicKey,
      })
      .signers([puppetKeypair])
      .rpc()

    await puppetMasterProgram.methods
      .pullStrings(new anchor.BN(42))
      .accounts({
        puppetProgram: puppetProgram.programId,
        puppet: puppetKeypair.publicKey,
      })
      .rpc()

    expect(
      (
        await puppetProgram.account.data.fetch(puppetKeypair.publicKey)
      ).data.toNumber()
    ).to.equal(42)
  })
})
```

and running `anchor test`.

## Privilege Extension

CPIs extend the privileges of the caller to the callee. The puppet account was passed as a mutable account to the puppet-master but it was still mutable in the puppet program as well (otherwise the `expect` in the test would've failed). The same applies to signatures.

If you want to prove this for yourself, add an `authority` field to the `Data` struct in the puppet program.

```rust
#[account]
pub struct Data {
    pub data: u64,
    pub authority: Pubkey
}
```

and adjust the `initialize` function:

```rust
pub fn initialize(ctx: Context<Initialize>, authority: Pubkey) -> Result<()> {
    ctx.accounts.puppet.authority = authority;
    Ok(())
}
```

Add `32` to the `space` constraint of the `puppet` field for the `Pubkey` field in the `Data` struct.

```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 8 + 32)]
    pub puppet: Account<'info, Data>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}
```

Then, adjust the `SetData` validation struct:

```rust
#[derive(Accounts)]
pub struct SetData<'info> {
    #[account(mut, has_one = authority)]
    pub puppet: Account<'info, Data>,
    pub authority: Signer<'info>
}
```

The `has_one` constraint checks that `puppet.authority = authority.key()`.

The puppet-master program now also needs adjusting:

```rust
use anchor_lang::prelude::*;
use puppet::cpi::accounts::SetData;
use puppet::program::Puppet;
use puppet::{self, Data};

declare_id!("HmbTLCmaGvZhKnn1Zfa1JVnp7vkMV4DYVxPLWBVoN65L");

#[program]
mod puppet_master {
    use super::*;
    pub fn pull_strings(ctx: Context<PullStrings>, data: u64) -> Result<()> {
        puppet::cpi::set_data(ctx.accounts.set_data_ctx(), data)
    }
}

#[derive(Accounts)]
pub struct PullStrings<'info> {
    #[account(mut)]
    pub puppet: Account<'info, Data>,
    pub puppet_program: Program<'info, Puppet>,
    // Even though the puppet program already checks that authority is a signer
    // using the Signer type here is still required because the anchor ts client
    // can not infer signers from programs called via CPIs
    pub authority: Signer<'info>
}

impl<'info> PullStrings<'info> {
    pub fn set_data_ctx(&self) -> CpiContext<'_, '_, '_, 'info, SetData<'info>> {
        let cpi_program = self.puppet_program.to_account_info();
        let cpi_accounts = SetData {
            puppet: self.puppet.to_account_info(),
            authority: self.authority.to_account_info()
        };
        CpiContext::new(cpi_program, cpi_accounts)
    }
}
```

Finally, change the test:

```ts
import * as anchor from '@project-serum/anchor'
import { Program } from '@project-serum/anchor'
import { Keypair } from '@solana/web3.js'
import { Puppet } from '../target/types/puppet'
import { PuppetMaster } from '../target/types/puppet_master'
import { expect } from 'chai'

describe('puppet', () => {
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider)

  const puppetProgram = anchor.workspace.Puppet as Program<Puppet>
  const puppetMasterProgram = anchor.workspace
    .PuppetMaster as Program<PuppetMaster>

  const puppetKeypair = Keypair.generate()
  const authorityKeypair = Keypair.generate()

  it('Does CPI!', async () => {
    await puppetProgram.methods
      .initialize(authorityKeypair.publicKey)
      .accounts({
        puppet: puppetKeypair.publicKey,
        user: provider.wallet.publicKey,
      })
      .signers([puppetKeypair])
      .rpc()

    await puppetMasterProgram.methods
      .pullStrings(new anchor.BN(42))
      .accounts({
        puppetProgram: puppetProgram.programId,
        puppet: puppetKeypair.publicKey,
        authority: authorityKeypair.publicKey,
      })
      .signers([authorityKeypair])
      .rpc()

    expect(
      (
        await puppetProgram.account.data.fetch(puppetKeypair.publicKey)
      ).data.toNumber()
    ).to.equal(42)
  })
})
```

The test passes because the signature that was given to the puppet-master by the authority was then extended to the puppet program which used it to check that the authority for the puppet account had signed the transaction.

> Privilege extension is convenient but also dangerous. If a CPI is unintentionally made to a malicious program,
> this program has the same privileges as the caller.
> Anchor protects you from CPIs to malicious programs with two measures.
> First, the `Program<'info, T>` type checks that the given account is the expected program `T`.
> Should you ever forget to use the `Program` type, the automatically generated cpi function
> (in the previous example this was `puppet::cpi::set_data`)
> also checks that the `cpi_program` argument equals the expected program.

## Reloading an Account

In the puppet program, the `Account<'info, T>` type is used for the `puppet` account. If a CPI edits an account of that type,
the caller's account does not change during the instruction.

You can easily see this for yourself by adding the following right after the `puppet::cpi::set_data(ctx.accounts.set_data_ctx(), data)` cpi call.

```rust
puppet::cpi::set_data(ctx.accounts.set_data_ctx(), data)?;
if ctx.accounts.puppet.data != 42 {
    panic!();
}
Ok(())
```

{% callout type="warning" title="Note" %}
Your test will fail. But why? After all the test used to pass, so the cpi definitely did change the `data` field to `42`.
{% /callout %}

The reason the `data` field has not been updated to `42` in the caller is that at the beginning of the instruction the `Account<'info, T>` type deserializes the incoming bytes into a new struct. This struct is no longer connected to the underlying data in the account. The CPI changes the data in the underlying account but since the struct in the caller has no connection to the underlying account the struct in the caller remains unchanged.

If you need to read the value of an account that has just been changed by a CPI, you can call its `reload` method which will re-deserialize the account. If you put `ctx.accounts.puppet.reload()?;` right after the cpi call, the test will pass again.

```rust
puppet::cpi::set_data(ctx.accounts.set_data_ctx(), data)?;
ctx.accounts.puppet.reload()?;
if ctx.accounts.puppet.data != 42 {
    panic!();
}
Ok(())
```

## Returning values from handler functions

The Anchor handler functions are capable of returning data using the Solana `set_return_data` and `get_return_data` syscalls. This data can be used in CPI callers and clients.

Instead of returning a `Result<()>`, consider this version of the `set_data` function from above which has been modified to return `Result<u64>`:

```rust
pub fn set_data(ctx: Context<SetData>, data: u64) -> Result<u64> {
    let puppet = &mut ctx.accounts.puppet;
    puppet.data = data;
    Ok(data)
}
```

Defining a return type that isn't the unit type `()` will cause Anchor to transparently call `set_return_data` with the given type (`u64` in this example) when this function is called. The return from the CPI call is wrapped in a struct to allow for lazy retrieval of this return data. E.g.

```rust
pub fn pull_strings(ctx: Context<PullStrings>, data: u64) -> Result<()> {
    let cpi_program = ctx.accounts.puppet_program.to_account_info();
    let cpi_accounts = SetData {
        puppet: ctx.accounts.puppet.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    let result = puppet::cpi::set_data(cpi_ctx, data)?;
    // The below statement calls sol_get_return and deserializes the result.
    // `return_data` contains the return from `set_data`,
    // which in this example is just `data`.
    let return_data = result.get();
    // ... do something with the `return_data` ...
}
```

{% callout type="warning" title="Note" %}
The type being returned must implement the `AnchorSerialize` and `AnchorDeserialize` traits, for example:
{% /callout %}

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct StructReturn {
    pub value: u64,
}
```

### Reading return data in the clients

It's even possible to use return values without CPIs. This may be useful if you're using a function to calculate a value that you need on the frontend without rewriting the code in the frontend.

Whether you're using a CPI or not, you can use the `view` function to read whatever was set last as return data in the transaction (`view` simulates the transaction and reads the `Program return` log).

For example:

```typescript
const returnData = await program.methods
  .calculate(someVariable)
  .accounts({
    acc: somePubkey,
    anotherAcc: someOtherPubkey,
  })
  .view()
```

### Return Data Size Limit Workarounds

The `set_return_data` and `get_return_data` syscalls are limited to 1024 bytes so it's worth briefly explaining the old workaround for CPI return values.

By using a CPI together with `reload` it's possible to simulate return values. One could imagine that instead of just setting the `data` field to `42` the puppet program did some calculation with the `42` and saved the result in `data`. The puppet-master can then call `reload` after the cpi and use the result of the puppet program's calculation.

## Programs as Signers

There's one more thing that can be done with CPIs. But for that, you need to first learn what PDAs are. We'll cover those in the next chapter.
