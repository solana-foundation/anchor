---
title: Program Derived Addresses
description: Anchor - Program Derived Addresses
---

Knowing how to use PDAs is one of the most important skills for Solana Programming.
They simplify the programming model and make programs more secure. So what are they?

---

PDAs (program derived addresses) are addresses with special properties.

Unlike normal addresses, PDAs are not public keys and therefore do not have an associated private key. There are two use cases for PDAs. They provide a mechanism to build hashmap-like structures on-chain and they allow programs to sign instructions.

## Creation of a PDA

Before we dive into how to use PDAs in anchor, here's a short explainer on what PDAs are.

PDAs are created by hashing a number of seeds the user can choose and the id of a program:

```rust
// pseudo code
let pda = hash(seeds, program_id);
```

The seeds can be anything. A pubkey, a string, an array of numbers etc.

There's a 50% chance that this hash function results in a public key (but PDAs are not public keys), so a bump has to be searched for so that we get a PDA:

```rust
// pseudo code
fn find_pda(seeds, program_id) {
  for bump in 0..256 {
    let potential_pda = hash(seeds, bump, program_id);
    if is_pubkey(potential_pda) {
      continue;
    }
    return (potential_pda, bump);
  }
  panic!("Could not find pda after 256 tries.");
}
```

It is technically possible that no bump is found within 256 tries but this probability is negligible.
If you're interested in the exact calculation of a PDA, check out the [`solana_program` source code](https://docs.rs/solana-program/latest/solana_program/pubkey/struct.Pubkey.html#method.find_program_address).

The first bump that results in a PDA is commonly called the "canonical bump". Other bumps may also result in a PDA but it's recommended to only use the canonical bump to avoid confusion.

## Using PDAs

We are now going to show you what you can do with PDAs and how to do it in Anchor!

### Hashmap-like structures using PDAs

Before we dive into the specifics of creating hashmaps in anchor, let's look at how to create a hashmap with PDAs in general.

#### Building hashmaps with PDAs

PDAs are hashed from the bump, a program id, but also a number of seeds which can be freely chosen by the user.
These seeds can be used to build hashmap-like structures on-chain.

For instance, imagine you're building an in-browser game and want to store some user stats. Maybe their level and their in-game name. You could create an account with a layout that looks like this:

```rust
pub struct UserStats {
  level: u16,
  name: String,
  authority: Pubkey
}
```

The `authority` would be the user the accounts belongs to.

This approach creates the following problem. It's easy to go from the user stats account to the user account address (just read the `authority` field) but if you just have the user account address (which is more likely), how do you find the user stats account? You can't. This is a problem because your game probably has instructions that require both the user stats account and its authority which means the client needs to pass those accounts into the instruction (for example, a `ChangeName` instruction). So maybe the frontend could store a mapping between a user's account address and a user's info address in local storage. This works until the user accidentally wipes their local storage.

With PDAs you can have a layout like this:

```rust
pub struct UserStats {
  level: u16,
  name: String,
  bump: u8
}
```

and encode the information about the relationship between the user and the user stats account in the address of the user stats account itself.

Reusing the pseudo code from above:

```rust
// pseudo code
let seeds = [b"user-stats", authority];
let (pda, bump) = find_pda(seeds, game_program_id);
```

When a user connects to your website, this pda calculation can be done client-side using their user account address as the `authority`. The resulting pda then serves as the address of the user's stats account. The `b"user-stats"` is added in case there are other account types that are also PDAs. If there were an inventory account, it could be inferred using these seeds:

```rust
let seeds = [b"inventory", authority];
```

To summarize, we have used PDAs to create a mapping between a user and their user stats account. There is no single hashmap object that exposes a `get` function. Instead, each value (the user stats address) can be found by using certain seeds ("user-stats" and the user account address) as inputs to the `find_pda` function.

#### How to build PDA hashmaps in Anchor

Continuing with the example from the previous sections, create a new workspace

```shell
anchor init game
```

and copy the following code

```rust
use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod game {
    use super::*;
    // handler function
    pub fn create_user_stats(ctx: Context<CreateUserStats>, name: String) -> Result<()> {
        let user_stats = &mut ctx.accounts.user_stats;
        user_stats.level = 0;
        if name.as_bytes().len() > 200 {
            // proper error handling omitted for brevity
            panic!();
        }
        user_stats.name = name;
        user_stats.bump = *ctx.bumps.get("user_stats").unwrap();
        Ok(())
    }
}

#[account]
pub struct UserStats {
    level: u16,
    name: String,
    bump: u8,
}

// validation struct
#[derive(Accounts)]
pub struct CreateUserStats<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    // space: 8 discriminator + 2 level + 4 name length + 200 name + 1 bump
    #[account(
        init,
        payer = user,
        space = 8 + 2 + 4 + 200 + 1, seeds = [b"user-stats", user.key().as_ref()], bump
    )]
    pub user_stats: Account<'info, UserStats>,
    pub system_program: Program<'info, System>,
}
```

In the account validation struct we use `seeds` together with `init` to create a PDA with the desired seeds.
Additionally, we add an empty `bump` constraint to signal to anchor that it should find the canonical bump itself.
Then, in the handler, we call `ctx.bumps.get("user_stats")` to get the bump anchor found and save it to the user stats
account as an extra property.

If we then want to use the created pda in a different instruction, we can add a new validation struct (This will check that the `user_stats` account is the pda created by running `hash(seeds, user_stats.bump, game_program_id)`):

```rust
// validation struct
#[derive(Accounts)]
pub struct ChangeUserName<'info> {
    pub user: Signer<'info>,
    #[account(mut, seeds = [b"user-stats", user.key().as_ref()], bump = user_stats.bump)]
    pub user_stats: Account<'info, UserStats>,
}
```

and another handler function:

```rust
// handler function (add this next to the create_user_stats function in the game module)
pub fn change_user_name(ctx: Context<ChangeUserName>, new_name: String) -> Result<()> {
    if new_name.as_bytes().len() > 200 {
        // proper error handling omitted for brevity
        panic!();
    }
    ctx.accounts.user_stats.name = new_name;
    Ok(())
}
```

Finally, let's add a test. Copy this into `game.ts`

```ts
import * as anchor from '@project-serum/anchor'
import { Program } from '@project-serum/anchor'
import { PublicKey } from '@solana/web3.js'
import { Game } from '../target/types/game'
import { expect } from 'chai'

describe('game', async () => {
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider)

  const program = anchor.workspace.Game as Program<Game>

  it('Sets and changes name!', async () => {
    const [userStatsPDA, _] = await PublicKey.findProgramAddress(
      [
        anchor.utils.bytes.utf8.encode('user-stats'),
        provider.wallet.publicKey.toBuffer(),
      ],
      program.programId
    )

    await program.methods
      .createUserStats('brian')
      .accounts({
        user: provider.wallet.publicKey,
        userStats: userStatsPDA,
      })
      .rpc()

    expect((await program.account.userStats.fetch(userStatsPDA)).name).to.equal(
      'brian'
    )

    await program.methods
      .changeUserName('tom')
      .accounts({
        user: provider.wallet.publicKey,
        userStats: userStatsPDA,
      })
      .rpc()

    expect((await program.account.userStats.fetch(userStatsPDA)).name).to.equal(
      'tom'
    )
  })
})
```

Exactly as described in the subchapter before this one, we use a `find` function to find the PDA. We can then use it just like a normal address. Well, almost. When we call `createUserStats`, we don't have to add the PDA to the `[signers]` array even though account creation requires a signature. This is because it is impossible to sign the transaction from outside the program as the PDA (it's not a public key so there is no private key to sign with). Instead, the signature is added when the CPI to the system program is made. We're going to explain how this works in the [Programs as Signers](#programs-as-signers) section.

#### Enforcing uniqueness

A subtle result of this hashmap structure is enforced uniqueness. When `init` is used with `seeds` and `bump`, it will always search for the canonical bump. This means that it can only be called once (because the 2nd time it's called the PDA will already be initialized). To illustrate how powerful enforced uniqueness is, consider a decentralized exchange program. In this program, anyone can create a new market for two assets. However, the program creators want liquidity to be concentrated so there should only be one market for every combination of two assets. This could be done without PDAs but would require a global account that saves all the different markets. Then upon market creation, the program would check whether the asset combination exists in the global market list. With PDAs this can be done in a much more straightforward way. Any market would simply be the PDA of the mint addresses of the two assets. The program would then check whether either of the two possible PDAs (because the market could've been created with the assets in reverse order) already exists.

### Programs as Signers

Creating PDAs requires them to sign the `createAccount` CPI of the system program. How does that work?

PDAs are not public keys so it's impossible for them to sign anything. However, PDAs can still pseudo sign CPIs.
In anchor, to sign with a pda you have to change `CpiContext::new(cpi_program, cpi_accounts)` to `CpiContext::new_with_signer(cpi_program, cpi_accounts, seeds)` where the `seeds` argument are the seeds _and_ the bump the PDA was created with.
When the CPI is invoked, for each account in `cpi_accounts` the Solana runtime will check whether`hash(seeds, current_program_id) == account address` is true. If yes, that account's `is_signer` flag will be turned to true.
This means a PDA derived from some program X, may only be used to sign CPIs that originate from that program X. This means that on a high level, PDA signatures can be considered program signatures.

This is great news because for many programs it is necessary that the program itself takes the authority over some assets.
For instance, lending protocol programs need to manage deposited collateral and automated market maker programs need to manage the tokens put into their liquidity pools.

Let's revisit the puppet workspace and add a PDA signature.

First, adjust the puppet-master code:

```rust
use anchor_lang::prelude::*;
use puppet::cpi::accounts::SetData;
use puppet::program::Puppet;
use puppet::{self, Data};

declare_id!("HmbTLCmaGvZhKnn1Zfa1JVnp7vkMV4DYVxPLWBVoN65L");

#[program]
mod puppet_master {
    use super::*;
    pub fn pull_strings(ctx: Context<PullStrings>, bump: u8, data: u64) -> Result<()> {
        let bump = &[bump][..];
        puppet::cpi::set_data(
            ctx.accounts.set_data_ctx().with_signer(&[&[bump][..]]),
            data,
        )
    }
}

#[derive(Accounts)]
pub struct PullStrings<'info> {
    #[account(mut)]
    pub puppet: Account<'info, Data>,
    pub puppet_program: Program<'info, Puppet>,
    /// CHECK: only used as a signing PDA
    pub authority: UncheckedAccount<'info>,
}

impl<'info> PullStrings<'info> {
    pub fn set_data_ctx(&self) -> CpiContext<'_, '_, '_, 'info, SetData<'info>> {
        let cpi_program = self.puppet_program.to_account_info();
        let cpi_accounts = SetData {
            puppet: self.puppet.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        CpiContext::new(cpi_program, cpi_accounts)
    }
}
```

The `authority` account is now an `UncheckedAccount` instead of a `Signer`. When the puppet-master is invoked, the `authority` pda is not a signer yet so we mustn't add a check for it. We just care about the puppet-master being able to sign so we don't add any additional seeds. Just a bump that is calculated off-chain and then passed to the function.

Finally, this is the new `puppet.ts`:

```ts
import * as anchor from '@project-serum/anchor'
import { Program } from '@project-serum/anchor'
import { Keypair, PublicKey } from '@solana/web3.js'
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

  it('Does CPI!', async () => {
    const [puppetMasterPDA, puppetMasterBump] =
      await PublicKey.findProgramAddress([], puppetMasterProgram.programId)

    await puppetProgram.methods
      .initialize(puppetMasterPDA)
      .accounts({
        puppet: puppetKeypair.publicKey,
        user: provider.wallet.publicKey,
      })
      .signers([puppetKeypair])
      .rpc()

    await puppetMasterProgram.methods
      .pullStrings(puppetMasterBump, new anchor.BN(42))
      .accounts({
        puppetProgram: puppetProgram.programId,
        puppet: puppetKeypair.publicKey,
        authority: puppetMasterPDA,
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

The `authority` is no longer a randomly generated keypair but a PDA derived from the puppet-master program. This means the puppet-master can sign with it which it does inside `pullStrings`. It's worth noting that our implementation also allows non-canonical bumps but again because we are only interested in being able to sign we don't care which bump is used.

> In some cases it's possible to reduce the number of accounts you need by making a PDA storing state also sign a CPI instead of defining a separate PDA to do that.

## PDAs: Conclusion

This section serves as a brief recap of the different things you can do with PDAs.

First, you can create hashmaps with them. We created a user stats PDA which was derived from the user address. This derivation linked the user address and the user stats account, allowing the latter to be easily found given the former.
Hashmaps also result in enforced uniqueness which can be used in many different ways, e.g. for only allowing one market per two assets in a decentralized exchange.

Secondly, PDAs can be used to allow programs to sign CPIs. This means that programs can be given control over assets which they then manage according to the rules defined in their code.

You can even combine these two use cases and use a PDA that's used in an instruction as a state account to also sign a CPI.

Admittedly, working with PDAs is one of the most challenging parts of working with Solana.
This is why in addition to our explanations here, we want to provide you with some further resources.

## Other Resources

- [Solana Cookbook](https://solanacookbook.com/core-concepts/pdas.html)
- [Pencilflips's twitter thread on PDAs](https://twitter.com/pencilflip/status/1455948263853600768?s=20&t=J2JXCwv395D7MNkX7a9LGw)
- [jarry xiao's talk on PDAs and CPIs](https://www.youtube.com/watch?v=iMWaQRyjpl4)
- [paulx's guide on everything Solana (covers much more than PDAs)](https://paulx.dev/blog/2021/01/14/programming-on-solana-an-introduction/)
