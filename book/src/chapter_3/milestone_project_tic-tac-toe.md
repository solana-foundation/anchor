# Milestone Project - Tic-Tac-Toe
You're now ready to build your first anchor project. Create a new anchor workspace with

```
anchor init tic-tac-toe
```

The program will have 2 instructions. First, we need to setup the game. We need to save who is playing it and create a board to play on. Then, the player take turns until there is a winner or a tie.

## Setting up the game

### State

Let's begin by thinking about which data we should store. Each game has players, turns, a board, and a game state. This game state describes whether the game is active, tied, or one of the two players won. We can save all this data in an account. This means that each new game will have its own account. Add the following to the bottom of the `lib.rs` file:
```rust,ignore
#[account]
pub struct Game {
    players: [Pubkey; 2],          // 64
    turn: u8,                      // 1
    board: [[Option<Sign>; 3]; 3], // 9 * (1 + 1) = 18
    state: GameState,              // 32 + 1
}
```
This is the game account. Next to the field definitions, you can see how many bytes each field requires. This will be very important later. Let's also add the `Sign` and the `GameState` type.
```rust,ignore
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
```rust,ignore
use num_derive::*;
use num_traits::*;
```

### The Setup Instruction

Before we write any game logic, we can add the instruction that will set up the game in its initial state. Rename the already existing instruction function and accounts struct to `setup_game` and `SetupGame` respectively. Now think about which accounts are needed to set up the game. Clearly, we need the game account. Before we can fill it with values, we need to create it. For that, we use the `init` constraint.
```rust,ignore
#[derive(Accounts)]
pub struct SetupGame<'info> {
    #[account(init)]
    pub game: Account<'info, Game>
}
```
`init` immediately shouts at us and tells us to add a payer. Why do we need it? Because `init` creates `rent-exempt` accounts and someone has to pay for that. Naturally, if we want to take money from someone, we should make them sign as well as mark their account as mutable.
```rust,ignore
#[derive(Accounts)]
pub struct SetupGame<'info> {
    #[account(init, payer = player_one)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player_one: Signer<'info>
}
```
`init` is not happy yet. It wants the system program to be inside the struct because `init` creates the game account by making a call to that program. So let's add it.
```rust,ignore
#[derive(Accounts)]
pub struct SetupGame<'info> {
    #[account(init, payer = player_one)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player_one: Signer<'info>,
    pub system_program: Program<'info, System>
}
```
There's one more thing to do to complete `SetupGame`. Every account is created with a fixed amount of space. `init` can try to infer how much space an account needs if it derives `Default`. So let's implement `Default` for `Game`
```rust,ignore
#[account]
#[derive(Default)] <-- add this
pub struct Game {...
```
 and `GameState`.
```rust,ignore
impl Default for GameState {
    fn default() -> Self {
        Self::Active
    }
}
```

And with this, `SetupGame` is complete and we can move on to the `setup_game` function. (If you like playing detective, you can pause here and try to figure out why what we just did will not work. Hint: Have a look at the [specification](https://borsh.io/) of the serialization library Anchor uses. If you cannot figure it out, don't worry. We are going to fix it very soon, together.)

Let's start by adding an argument to the `setup_game` function.
```rust,ignore
pub fn setup_game(ctx: Context<SetupGame>, player_two: Pubkey) -> ProgramResult {
    Ok(())
}
```
Why didn't we just add `player_two` as an account in the accounts struct? There are two reasons for this.First, adding it there requires a little more space in the transaction that saves whether the account is writable and whether it's a signer. But we care about neither of that, we just want the address. This brings us to the second and more important reason. A transaction can have effects on other transaction that are processed at the same time in the network if they share accounts. For example, if we add `player_two` to the accounts struct, during our transaction, no other transaction can edit `player_two`'s account. Therefore, we block all other transactions that want to edit `player_two`'s account, even though we neither want to read from nor write to the account. We just care about its address!

Finish the instruction function by setting the game to its initial values:
```rust,ignore
pub fn setup_game(ctx: Context<SetupGame>, player_two: Pubkey) -> ProgramResult {
    let game = &mut ctx.accounts.game;
    game.players = [ctx.accounts.player_one.key(), player_two];
    game.turn = 1;
    Ok(())
}
```

Now, run `anchor build`. On top of compiling your program, this command creates an [IDL](https://en.wikipedia.org/wiki/Interface_description_language) for your program. You can find it in `target/idl`. The anchor typescript client can automatically parse this IDL and generate functions based on it. What this means is that each anchor program gets its own typescript client for free! (Technically, you don't have to call anchor build before testing. `anchor test` will do it for you.)

### Testing the Setup Instruction

Time to test our code! Head over into the `tests` folder in the root directory. Open the `tic-tac-toe.ts` file and remove the existing `it` test. Then, put the following into the `describe` section:
```typescript
  it('setup game!', async() => {
    const gameKeypair = anchor.web3.Keypair.generate();
    const playerOne = program.provider.wallet;
    const playerTwo = anchor.web3.Keypair.generate();
    await program.rpc.setupGame(playerTwo.publicKey, {
      accounts: {
        game: gameKeypair.publicKey,
        playerOne: playerOne.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId
      },
      signers: [gameKeypair]
    });

    let gameState = await program.account.game.fetch(gameKeypair.publicKey);
    expect(gameState.turn).to.equal(1);
    expect(gameState.players)
      .to
      .eql([playerOne.publicKey, playerTwo.publicKey]);
    expect(gameState.state).to.eql({ active: {} });
    expect(gameState.board)
      .to
      .eql([[null,null,null],[null,null,null],[null,null,null]]);
  });
```

and add this to the top of your file:
```typescript
import { expect } from 'chai';
```

The test begins by creating some keypairs. Importantly, `playerOne` is not a keypair but the wallet of the program's provider. The provider details are defined in the `Anchor.toml` file in the root of the project.
Then, we send the transaction. Because the anchor typescript client has parsed the IDL, all transaction inputs have types. If you remove one of the accounts for example, typescript will complain. 
The structure of the transaction function is as follows. First come the instruction arguments. For this function, the public key of the second player. Then come the accounts. Lastly, we add a signers array. We have to add the `gameKeypair` here because whenever an account gets created, it has to sign its creation transaction. We don't have to add `playerOne` even though we gave it the `Signer` type in the program because it is the program provider and therefore signs the transaction by default.

After the transaction returns, we can fetch the state of the game account. You can fetch account state using the `program.account` namespace. 
Finally, we verify the game has been set up properly. Anchor's typescript client deserializes rust enums like this: `{ active: {}}` for a fieldless variant and `{ won: { winner: Pubkey }}` for a variant with fields. The `None` variant of `Option` becomes `null`. The `Some(x)` variant becomes whatever `x` deserializes to.

Now, run `anchor test`. This starts up (and subsequently shuts down) a local validator (make sure you don't have one running) and runs your tests using the test script defined in `Anchor.toml`.

## Playing the game

### The Play Instruction

The `Play` accounts struct is straightforward. We need the game and a player:
```rust,ignore
#[derive(Accounts)]
pub struct Play<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
    pub player: Signer<'info>,
}
```

`player` needs to sign or someone else could play for the player.

Next, add the game logic:

```rust,ignore
impl Game {
    pub fn is_active(&self) -> bool {
        self.state == GameState::Active
    }

    fn current_player_index(&self) -> usize {
        ((self.turn - 1) % 2) as usize
    }

    pub fn current_player(&self) -> Pubkey {
        self.players[self.current_player_index()]
    }

    pub fn play(&mut self, tile: &Tile) -> ProgramResult {
        if !self.is_active() {
            return Err(TicTacToeError::GameAlreadyOver.into());
        }
        match tile {
            tile
            @ Tile {
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

        if let GameState::Active = self.state {
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
1. return error if game is over or
return error if given row or column are outside the 3x3 board or
return error if tile on board is already set
2. determine current player and set tile to X or O
3. update game state
4. if game is still active, increase the turn

Currently, the code doesn't compile because we need to add the `Tile`
```rust,ignore
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct Tile {
    row: u8,
    column: u8,
}
```

and the `TicTacToeError` type.
```rust,ignore
#[error]
pub enum TicTacToeError {
    TileOutOfBounds,
    TileAlreadySet,
    GameAlreadyOver,
    NotPlayersTurn,
}
```

Finally, we can add the `play` function inside the program module.
```rust,ignore
pub fn play(ctx: Context<Play>, tile: Tile) -> ProgramResult {
    let game = &mut ctx.accounts.game;

    require!(
        game.current_player() == ctx.accounts.player.key(),
        TicTacToeError::NotPlayersTurn
    );

    game.play(&tile)
}
```

We've checked in the accounts struct that the `player` account has signed the transaction, but we do not check that it is the `player` we expect. That's what the `require` check in `play` is for.

### Testing the Play Instruction

Testing the `play` instruction works the exact same way. To avoid repeating yourself, create a helper function at the top of the test file:
```typescript
async function play(program, game, player,
    tile, expectedTurn, expectedGameState, expectedBoard) {
  await program.rpc.play(tile, {
    accounts: {
      player: player.publicKey,
      game
    },
    signers: player instanceof (anchor.Wallet as any) ? [] : [player]
  });

  const gameState = await program.account.game.fetch(game);
  expect(gameState.turn).to.equal(expectedTurn);
  expect(gameState.state).to.eql(expectedGameState);
  expect(gameState.board)
    .to
    .eql(expectedBoard);
}
```

You can create then a new `it` test, setup the game like in the previous test, but then keep calling the `play` function you just added to simulate a complete run of the game. Let's begin with the first turn:
```typescript
it('player one wins', async() => {
    const gameKeypair = anchor.web3.Keypair.generate();
    const playerOne = program.provider.wallet;
    const playerTwo = anchor.web3.Keypair.generate();
    await program.rpc.setupGame(playerTwo.publicKey, {
      accounts: {
        game: gameKeypair.publicKey,
        playerOne: playerOne.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId
      },
      signers: [gameKeypair]
    });

    let gameState = await program.account.game.fetch(gameKeypair.publicKey);
    expect(gameState.turn).to.equal(1);
    expect(gameState.players)
      .to
      .eql([playerOne.publicKey, playerTwo.publicKey]);
    expect(gameState.state).to.eql({ active: {} });
    expect(gameState.board)
      .to
      .eql([[null,null,null],[null,null,null],[null,null,null]]);

    await play(
      program,
      gameKeypair.publicKey,
      playerOne,
      {row: 0, column: 0},
      2,
      { active: {}, },
      [
        [{x:{}},null,null],
        [null,null,null],
        [null,null,null]
      ]
    );
});
```

Now run `anchor test` again and you will be greeted with an error:
```
Error: 3004: Failed to serialize the account
```
What to do? We know that it happens during play, because our `setupGame` test runs fine.
Also, it says `serialize`, not `deserialize`. So after our logic runs and anchor tries to save all the data, there is an error.
What this means most of the time is that the account is too small to hold all its data and this is also the problem here.

Let's have a look at the `Game` struct again and the way we created it:
```rust,ignore
#[account]
#[derive(Default)]
pub struct Game {
    players: [Pubkey; 2],          // 64
    turn: u8,                      // 1
    board: [[Option<Sign>; 3]; 3], // 9 * (1 + 1) = 18
    state: GameState,              // 32 + 1
}

...
#[account(init, payer = player_one)]
pub game: Account<'info, Game>,
...

```

Remember that we implemented `Default` for `Game` because `init` can try to infer the correct space requirements based on `Default`, "try" being the operative word. What happens if we don't specify an explicit `space` requirement for the account is that anchor will call `default` on the account and convert it to a vector using borsh. It then uses the length of that vector as the space for the account.
Let's walk through our example step by step using the [borsh specification](https://borsh.io/). The comments show us the space requirements that we must get, that is, the largest the given type can become.
- Pubkey as a vector has a length of `32` so `2*32 = 64` ✅
- u8 as a vector has a length of `1` so `1 = 1` ✅
- board's default (`9 * None`) as a vector has a length of `9 != 18` ❌
- state's default as a vector is `1 != 33` ❌

We have found out that `init` currently only allocates 75 bytes for our account data but the account can grow to (64 + 1 + 18 + 33) = 116 bytes.
We can add this number to our Game impl like this:
```rust,ignore
impl Game {
    const MAXIMUM_SIZE: usize = 116;

    ... // other functions
}
```

```rust,ignore
...
#[account(init, payer = player_one, space = Game::MAXIMUM_SIZE + 8)]
pub game: Account<'info, Game>,
...
```
In addition to the game's size, we have to add another `8` to the space. This is space for the `discriminator` which anchor sets automatically. In short, the discriminator is how anchor can differentiate between different accounts of the same program.

> (What about using `mem::size_of<Game>()`? This almost works but not quite. The issue is that borsh will always serialize an option as 1 byte for the variant identifier and then additional x bytes for the content if it's Some. Rust uses null-pointer optimization to make Option's variant identifier 0 bytes when it can, so an option is sometimes just as big as its contents. This is the case with `Sign`. This means the `MAXIMUM_SIZE` could be expressed as `mem::size_of<Game>() + 9`.)

Running `anchor test` should work now. You can finish writing the test by yourself. Try to simulate a win and a tie!

Proper testing also includes tests that try to exploit the contract. You can check whether you've protected yourself properly by calling `play` with unexpected parameters. For example:
```typescript
try {
  await play(
    program,
    gameKeypair.publicKey,
    playerTwo,
    {row: 5, column: 1}, // out of bounds row
    4,
    { active: {}, },
    [
      [{x:{}},{x: {}},null],
      [{o:{}},null,null],
      [null,null,null]
    ]
  );
  // we use this to make sure we definitely throw an error
  chai.assert(false, "should've failed but didn't ");
} catch (error) {
  expect(error.code).to.equal(6000);
}
```

## Deployment

Solana has three main clusters: `mainnet-beta`, `devnet`, and `testnet`.
For developers, `devnet` and `mainnet-beta` are the most interesting. `devnet` is where you test your application in a more realistic environment than `localnet`. `testnet` is mostly for validators.

We are going to deploy on `devnet`.

Here is your deployment checklist 🚀

1. run `anchor build`. Your program keypair is now in `target/deploy`. Keep this secret. You can reuse it on all clusters.
2. run `solana address -k target/deploy/tic_tac_toe-keypair.json` and copy the address into your `declare_id!` macro at the top of `lib.rs`.
3. run `anchor build` again. This step is is necessary to include our new program id in the binary.
4. change the `provider.cluster` variable in `Anchor.toml` to `devnet`.
5. run `anchor deploy`
6. run `anchor test`

There is more to deployments than this e.g. understanding how the BPFLoader works, how to manage keys, how to upgrade your programs and more. Keep reading to learn more!

Well done! You've finished the essentials section. You can now move on to the more advanced parts of Anchor.