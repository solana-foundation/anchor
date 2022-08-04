---
title: Project - Tic-Tac-Toe
description: Anchor - Milestone Project - Tic-Tac-Toe
---

> [Program Code](https://github.com/coral-xyz/anchor/tree/master/docs/programs/tic-tac-toe)

You're now ready to build your first anchor project. Create a new anchor workspace with

```shell
anchor init tic-tac-toe
```

The program will have 2 instructions. First, we need to setup the game. We need to save who is playing it and create a board to play on. Then, the players take turns until there is a winner or a tie.

We recommend keeping programs in a single `lib.rs` file until they get too big. We would not split up this project into multiple files either but there is a section at the end of this chapter that explains how to do it for this and other programs.

## Setting up the game

### State

Let's begin by thinking about what data we should store. Each game has players, turns, a board, and a game state. This game state describes whether the game is active, tied, or one of the two players won. We can save all this data in an account. This means that each new game will have its own account. Add the following to the bottom of the `lib.rs` file:

```rust
#[account]
pub struct Game {
    players: [Pubkey; 2],          // (32 * 2)
    turn: u8,                      // 1
    board: [[Option<Sign>; 3]; 3], // 9 * (1 + 1) = 18
    state: GameState,              // 32 + 1
}
```

This is the game account. Next to the field definitions, you can see how many bytes each field requires. This will be very important later. Let's also add the `Sign` and the `GameState` type.

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum GameState {
    Active,
    Tie,
    Won { winner: Pubkey },
}

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    FromPrimitive,
    ToPrimitive,
    Copy,
    Clone,
    PartialEq,
    Eq
)]
pub enum Sign {
    X,
    O,
}
```

Both `GameState` and `Sign` derive some traits. `AnchorSerialize` and `AnchorDeserialize` are the crucial ones. All types that are used in types that are marked with `#[account]` must implement these two traits (or be marked with `#[account]` themselves). All other traits are important to our game logic and we are going to use them later. Generally, it is good practice to derive even more traits to make the life of others trying to interface with your program easier (see [Rust's API guidelines](https://rust-lang.github.io/api-guidelines/interoperability.html#types-eagerly-implement-common-traits-c-common-traits)) but for brevity's sake, we are not going to do that in this guide.

This won't quite work yet because `FromPrimitive` and `ToPrimitive` are unknown. Go to the `Cargo.toml` file right outside `src` (not the one at the root of the workspace) and add these two dependencies:

```toml
num-traits = "0.2"
num-derive = "0.3"
```

Then, import them at the top of `lib.rs`:

```rust
use num_derive::*;
use num_traits::*;
```

Now add the game logic:

```rust
impl Game {
    pub const MAXIMUM_SIZE: usize = (32 * 2) + 1 + (9 * (1 + 1)) + (32 + 1);

    pub fn start(&mut self, players: [Pubkey; 2]) -> Result<()> {
        require_eq!(self.turn, 0, TicTacToeError::GameAlreadyStarted);
        self.players = players;
        self.turn = 1;
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.state == GameState::Active
    }

    fn current_player_index(&self) -> usize {
        ((self.turn - 1) % 2) as usize
    }

    pub fn current_player(&self) -> Pubkey {
        self.players[self.current_player_index()]
    }

    pub fn play(&mut self, tile: &Tile) -> Result<()> {
        require!(self.is_active(), TicTacToeError::GameAlreadyOver);

        match tile {
            tile @ Tile {
                row: 0..=2,
                column: 0..=2,
            } => match self.board[tile.row as usize][tile.column as usize] {
                Some(_) => return Err(TicTacToeError::TileAlreadySet.into()),
                None => {
                    self.board[tile.row as usize][tile.column as usize] =
                        Some(Sign::from_usize(self.current_player_index()).unwrap());
                }
            },
            _ => return Err(TicTacToeError::TileOutOfBounds.into()),
        }

        self.update_state();

        if GameState::Active == self.state {
            self.turn += 1;
        }

        Ok(())
    }

    fn is_winning_trio(&self, trio: [(usize, usize); 3]) -> bool {
        let [first, second, third] = trio;
        self.board[first.0][first.1].is_some()
            && self.board[first.0][first.1] == self.board[second.0][second.1]
            && self.board[first.0][first.1] == self.board[third.0][third.1]
    }

    fn update_state(&mut self) {
        for i in 0..=2 {
            // three of the same in one row
            if self.is_winning_trio([(i, 0), (i, 1), (i, 2)]) {
                self.state = GameState::Won {
                    winner: self.current_player(),
                };
                return;
            }
            // three of the same in one column
            if self.is_winning_trio([(0, i), (1, i), (2, i)]) {
                self.state = GameState::Won {
                    winner: self.current_player(),
                };
                return;
            }
        }

        // three of the same in one diagonal
        if self.is_winning_trio([(0, 0), (1, 1), (2, 2)])
            || self.is_winning_trio([(0, 2), (1, 1), (2, 0)])
        {
            self.state = GameState::Won {
                winner: self.current_player(),
            };
            return;
        }

        // reaching this code means the game has not been won,
        // so if there are unfilled tiles left, it's still active
        for row in 0..=2 {
            for column in 0..=2 {
                if self.board[row][column].is_none() {
                    return;
                }
            }
        }

        // game has not been won
        // game has no more free tiles
        // -> game ends in a tie
        self.state = GameState::Tie;
    }
}
```

We are not going to explore this code in detail together because it's rather simple rust code. It's just tic-tac-toe after all! Roughly, what happens when `play` is called:

1. Return error if game is over or
   return error if given row or column are outside the 3x3 board or
   return error if tile on board is already set
2. Determine current player and set tile to X or O
3. Update game state
4. If game is still active, increase the turn

Currently, the code doesn't compile because we need to add the `Tile`

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct Tile {
    row: u8,
    column: u8,
}
```

and the `TicTacToeError` type.

```rust
#[error_code]
pub enum TicTacToeError {
    TileOutOfBounds,
    TileAlreadySet,
    GameAlreadyOver,
    NotPlayersTurn,
    GameAlreadyStarted
}
```

### The Setup Instruction

Before we write any game logic, we can add the instruction that will set up the game in its initial state. Rename the already existing instruction function and accounts struct to `setup_game` and `SetupGame` respectively. Now think about which accounts are needed to set up the game. Clearly, we need the game account. Before we can fill it with values, we need to create it. For that, we use the `init` constraint.

```rust
#[derive(Accounts)]
pub struct SetupGame<'info> {
    #[account(init)]
    pub game: Account<'info, Game>
}
```

`init` immediately shouts at us and tells us to add a payer. Why do we need it? Because `init` creates `rent-exempt` accounts and someone has to pay for that. Naturally, if we want to take money from someone, we should make them sign as well as mark their account as mutable.

```rust
#[derive(Accounts)]
pub struct SetupGame<'info> {
    #[account(init, payer = player_one)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player_one: Signer<'info>
}
```

`init` is not happy yet. It wants the system program to be inside the struct because `init` creates the game account by making a call to that program. So let's add it.

```rust
#[derive(Accounts)]
pub struct SetupGame<'info> {
    #[account(init, payer = player_one)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player_one: Signer<'info>,
    pub system_program: Program<'info, System>
}
```

There's one more thing to do to complete `SetupGame`. Every account is created with a fixed amount of space, so we have to add this space to the instruction as well. This is what the comments next to the `Game` struct indicated.

```rust
#[derive(Accounts)]
pub struct SetupGame<'info> {
    #[account(init, payer = player_one, space = 8 + Game::MAXIMUM_SIZE)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player_one: Signer<'info>,
    pub system_program: Program<'info, System>
}
```

Let us briefly explain how we arrived at the `Game::MAXIMUM_SIZE`. Anchor uses the [borsh](https://borsh.io) specification to (de)serialize its state accounts.

- Pubkey has a length of `32` bytes so `2*32 = 64`
- u8 as a vector has a length of `1`
- the `board` has a length of (`9 * (1 + 1)`). We know the board has 9 tiles (-> `9`) of type `Option` which borsh serializes with 1 byte (set to `1` for Some and `0` for None) plus the size of whatever's in the `Option`. In this case, it's a simple enum with types that don't hold more types so the maximum size of the enum is also just `1` (for its discriminant). In total that means we get `9 (tiles) * (1 (Option) + 1(Sign discriminant))`.
- `state` is also an enum so we need `1` byte for the discriminant. We have to init the account with the maximum size and the maximum size of an enum is the size of its biggest variant. In this case that's the `winner` variant which holds a Pubkey. A Pubkey is `32` bytes long so the size of `state` is `1 (discriminant) + 32 (winner pubkey)` (`MAXIMUM_SIZE` is a [`const`](https://doc.rust-lang.org/std/keyword.const.html) variable so specifying it in terms of a sum of the sizes of `Game`'s members' fields does not incur any runtime cost).

In addition to the game's size, we have to add another 8 to the space. This is space for the internal discriminator which anchor sets automatically. In short, the discriminator is how anchor can differentiate between different accounts of the same program. For more information, check out the Anchor space reference.

> [Anchor Space Reference](./../anchor_references/space.md)

> (What about using `mem::size_of<Game>()`? This almost works but not quite. The issue is that borsh will always serialize an option as 1 byte for the variant identifier and then additional x bytes for the content if it's Some. Rust uses null-pointer optimization to make Option's variant identifier 0 bytes when it can, so an option is sometimes just as big as its contents. This is the case with `Sign`. This means the `MAXIMUM_SIZE` could also be expressed as `mem::size_of<Game>() + 9`.)

And with this, `SetupGame` is complete and we can move on to the `setup_game` function. (If you like playing detective, you can pause here and try to figure out why what we just did will not work. Hint: Have a look at the [specification](https://borsh.io/) of the serialization library Anchor uses. If you cannot figure it out, don't worry. We are going to fix it very soon, together.)

Let's start by adding an argument to the `setup_game` function.

```rust
pub fn setup_game(ctx: Context<SetupGame>, player_two: Pubkey) -> Result<()> {

}
```

Why didn't we just add `player_two` as an account in the accounts struct? There are two reasons for this. First, adding it there requires a little more space in the transaction that saves whether the account is writable and whether it's a signer. But we care about neither the mutability of the account nor whether it's a signer. We just need its address. This brings us to the second and more important reason: Simultaneous network transactions can affect each other if they share the same accounts. For example, if we add `player_two` to the accounts struct, during our transaction, no other transaction can edit `player_two`'s account. Therefore, we block all other transactions that want to edit `player_two`'s account, even though we neither want to read from nor write to the account. We just care about its address!

Finish the instruction function by setting the game to its initial values:

```rust
pub fn setup_game(ctx: Context<SetupGame>, player_two: Pubkey) -> Result<()> {
    ctx.accounts.game.start([ctx.accounts.player_one.key(), player_two])
}
```

Now, run `anchor build`. On top of compiling your program, this command creates an [IDL](https://en.wikipedia.org/wiki/Interface_description_language) for your program. You can find it in `target/idl`. The anchor typescript client can automatically parse this IDL and generate functions based on it. What this means is that each anchor program gets its own typescript client for free! (Technically, you don't have to call `anchor build` before testing. `anchor test` will do it for you.)

### Testing the Setup Instruction

Time to test our code! Head over into the `tests` folder in the root directory. Open the `tic-tac-toe.ts` file and remove the existing `it` test. Then, put the following into the `describe` section:

```typescript
it('setup game!', async () => {
  const gameKeypair = anchor.web3.Keypair.generate()
  const playerOne = (program.provider as anchor.AnchorProvider).wallet
  const playerTwo = anchor.web3.Keypair.generate()
  await program.methods
    .setupGame(playerTwo.publicKey)
    .accounts({
      game: gameKeypair.publicKey,
      playerOne: playerOne.publicKey,
    })
    .signers([gameKeypair])
    .rpc()

  let gameState = await program.account.game.fetch(gameKeypair.publicKey)
  expect(gameState.turn).to.equal(1)
  expect(gameState.players).to.eql([playerOne.publicKey, playerTwo.publicKey])
  expect(gameState.state).to.eql({ active: {} })
  expect(gameState.board).to.eql([
    [null, null, null],
    [null, null, null],
    [null, null, null],
  ])
})
```

and add this to the top of your file:

```typescript
import { expect } from 'chai'
```

> When you adjust your test files it may happen that you'll see errors everywhere.
> This is likely because the test file is looking for types from your program that haven't been generated yet.
> To generate them, run `anchor build`. This builds the program and creates the idl and typescript types.

The test begins by creating some keypairs. Importantly, `playerOne` is not a keypair but the wallet of the program's provider. The provider details are defined in the `Anchor.toml` file in the root of the project. The provider serves as the keypair that pays for (and therefore signs) all transactions.
Then, we send the transaction.
The structure of the transaction function is as follows: First come the instruction arguments. For this function, the public key of the second player. Then come the accounts. Lastly, we add a signers array. We have to add the `gameKeypair` here because whenever an account gets created, it has to sign its creation transaction. We don't have to add `playerOne` even though we gave it the `Signer` type in the program because it is the program provider and therefore signs the transaction by default.
We did not have to specify the `system_program` account. This is because anchor recognizes this account and is able to infer it. This is also true for other known accounts such as the `token_program` or the `rent` sysvar account.

After the transaction returns, we can fetch the state of the game account. You can fetch account state using the `program.account` namespace.
Finally, we verify the game has been set up properly by comparing the actual state and the expected state. To learn how Anchor maps the Rust types to the js/ts types, check out the [Javascript Anchor Types Reference](./../anchor_references/javascript_anchor_types_reference.md).

Now, run `anchor test`. This starts up (and subsequently shuts down) a local validator (make sure you don't have one running before) and runs your tests using the test script defined in `Anchor.toml`.

> If you get the error `Error: Unable to read keypair file` when running the test, you likely need to generate a Solana keypair using `solana-keygen new`.

## Playing the game

### The Play Instruction

The `Play` accounts struct is straightforward. We need the game and a player:

```rust
#[derive(Accounts)]
pub struct Play<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
    pub player: Signer<'info>,
}
```

`player` needs to sign or someone else could play for the player.

Finally, we can add the `play` function inside the program module.

```rust
pub fn play(ctx: Context<Play>, tile: Tile) -> Result<()> {
    let game = &mut ctx.accounts.game;

    require_keys_eq!(
        game.current_player(),
        ctx.accounts.player.key(),
        TicTacToeError::NotPlayersTurn
    );

    game.play(&tile)
}
```

We've checked in the accounts struct that the `player` account has signed the transaction, but we do not check that it is the `player` we expect. That's what the `require_keys_eq` check in `play` is for.

### Testing the Play Instruction

Testing the `play` instruction works the exact same way. To avoid repeating yourself, create a helper function at the top of the test file:

```typescript
async function play(
  program: Program<TicTacToe>,
  game,
  player,
  tile,
  expectedTurn,
  expectedGameState,
  expectedBoard
) {
  await program.methods
    .play(tile)
    .accounts({
      player: player.publicKey,
      game,
    })
    .signers(player instanceof (anchor.Wallet as any) ? [] : [player])
    .rpc()

  const gameState = await program.account.game.fetch(game)
  expect(gameState.turn).to.equal(expectedTurn)
  expect(gameState.state).to.eql(expectedGameState)
  expect(gameState.board).to.eql(expectedBoard)
}
```

You can create then a new `it` test, setup the game like in the previous test, but then keep calling the `play` function you just added to simulate a complete run of the game. Let's begin with the first turn:

```typescript
it('player one wins', async () => {
  const gameKeypair = anchor.web3.Keypair.generate()
  const playerOne = program.provider.wallet
  const playerTwo = anchor.web3.Keypair.generate()
  await program.methods
    .setupGame(playerTwo.publicKey)
    .accounts({
      game: gameKeypair.publicKey,
      playerOne: playerOne.publicKey,
    })
    .signers([gameKeypair])
    .rpc()

  let gameState = await program.account.game.fetch(gameKeypair.publicKey)
  expect(gameState.turn).to.equal(1)
  expect(gameState.players).to.eql([playerOne.publicKey, playerTwo.publicKey])
  expect(gameState.state).to.eql({ active: {} })
  expect(gameState.board).to.eql([
    [null, null, null],
    [null, null, null],
    [null, null, null],
  ])

  await play(
    program,
    gameKeypair.publicKey,
    playerOne,
    { row: 0, column: 0 },
    2,
    { active: {} },
    [
      [{ x: {} }, null, null],
      [null, null, null],
      [null, null, null],
    ]
  )
})
```

and run `anchor test`.

You can finish writing the test by yourself (or check out [the reference implementation](https://github.com/coral-xyz/anchor/tree/master/docs/programs/tic-tac-toe)). Try to simulate a win and a tie!

Proper testing also includes tests that try to exploit the contract. You can check whether you've protected yourself properly by calling `play` with unexpected parameters. You can also familiarize yourself with the returned `AnchorErrors` this way. For example:

```typescript
try {
  await play(
    program,
    gameKeypair.publicKey,
    playerTwo,
    { row: 5, column: 1 }, // ERROR: out of bounds row
    4,
    { active: {} },
    [
      [{ x: {} }, { x: {} }, null],
      [{ o: {} }, null, null],
      [null, null, null],
    ]
  )
  // we use this to make sure we definitely throw an error
  chai.assert(false, "should've failed but didn't ")
} catch (_err) {
  expect(_err).to.be.instanceOf(AnchorError)
  const err: AnchorError = _err
  expect(err.error.errorCode.number).to.equal(6000)
}
```

or

```typescript
try {
  await play(
    program,
    gameKeypair.publicKey,
    playerOne, // ERROR: same player in subsequent turns

    // change sth about the tx because
    // duplicate tx that come in too fast
    // after each other may get dropped
    { row: 1, column: 0 },
    2,
    { active: {} },
    [
      [{ x: {} }, null, null],
      [null, null, null],
      [null, null, null],
    ]
  )
  chai.assert(false, "should've failed but didn't ")
} catch (_err) {
  expect(_err).to.be.instanceOf(AnchorError)
  const err: AnchorError = _err
  expect(err.error.errorCode.code).to.equal('NotPlayersTurn')
  expect(err.error.errorCode.number).to.equal(6003)
  expect(err.program.equals(program.programId)).is.true
  expect(err.error.comparedValues).to.deep.equal([
    playerTwo.publicKey,
    playerOne.publicKey,
  ])
}
```

## Deployment

Solana has three main clusters: `mainnet-beta`, `devnet`, and `testnet`.
For developers, `devnet` and `mainnet-beta` are the most interesting. `devnet` is where you test your application in a more realistic environment than `localnet`. `testnet` is mostly for validators.

We are going to deploy on `devnet`.

Here is your deployment checklist 🚀

1. Run `anchor build`. Your program keypair is now in `target/deploy`. Keep this keypair secret. You can reuse it on all clusters.
2. Run `anchor keys list` to display the keypair's public key and copy it into your `declare_id!` macro at the top of `lib.rs`.
3. Run `anchor build` again. This step is necessary to include the new program id in the binary.
4. Change the `provider.cluster` variable in `Anchor.toml` to `devnet`.
5. Run `anchor deploy`
6. Run `anchor test`

There is more to deployments than this e.g. understanding how the BPFLoader works, how to manage keys, how to upgrade your programs and more. Keep reading to learn more!

## Program directory organization

> [Program Code](https://github.com/coral-xyz/anchor/tree/master/docs/programs/tic-tac-toe)

Eventually, some programs become too big to keep them in a single file and it makes sense to break them up.

Splitting a program into multiple files works almost the exact same way as splitting up a regular rust program, so if you haven't already, now is the time to read all about that in the [rust book](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html).

We recommend the following directory structure (using the tic-tac-toe program as an example):

```
.
+-- lib.rs
+-- errors.rs
+-- instructions
|   +-- play.rs
|   +-- setup_game.rs
|   +-- mod.rs
+-- state
|   +-- game.rs
|   +-- mod.rs
```

The crucial difference to a normal rust layout is the way that instructions have to be imported. The `lib.rs` file has to import each instruction module with a wildcard import (e.g. `use instructions::play::*;`). This has to be done because the `#[program]` macro depends on generated code inside each instruction file.

To make the imports shorter you can re-export the instruction modules in the `mod.rs` file in the instructions directory with the `pub use` syntax and then import all instructions in the `lib.rs` file with `use instructions::*;`.

Well done! You've finished the essentials section. You can now move on to the more advanced parts of Anchor.
