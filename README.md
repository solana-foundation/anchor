# Anchor ⚓

Anchor is a DSL for Solana's [Sealevel](https://medium.com/solana-labs/sealevel-parallel-processing-thousands-of-smart-contracts-d814b378192) runtime, providing a safer and more convenient programming model to the Solana developer along with the ability to extract an [IDL](https://en.wikipedia.org/wiki/Interface_description_language) from source.

## Note

* **Anchor is in active development, so all APIs are subject to change.**
* **This code is unaudited. Use at your own risk.**

## Goal

It's primary goal is to add safety to one's programs by providing the ability to more easily reason about program inputs. Because Solana programs are stateless, a transaction must specify accounts to be executed. And because an untrusted client specifies those accounts, a program must responsibily validate all input to the program to ensure it is what it claims to be (in addition to any instruction specific access control the program needs to do). This is particularly burdensome when there are lots of dependencies between accounts, leading to repetitive [boilerplate](https://github.com/project-serum/serum-dex/blob/master/registry/src/access_control.rs) code for account validation along with the ability to easily shoot oneself in the foot by forgetting to validate any particular account.

For example, one could imagine easily writing a faulty SPL token program that forgets to check the owner of a token account actually matches the owner on the account. So one must write an `if` statement to check for all such conditions. Instead, one can use the Anchor DSL to do these checks.

## Example

See a full example [here](./examples/basic/src/lib.rs) along with a generated [IDL](./examples/basic/idl.json).

```rust
// Program instruction handler.

#[program]
mod example {
    pub fn create_root(ctx: Context<Initialize>, initial_data: u64) {
      let root = &mut ctx.accounts.root;
      root.account.initialized = true;
      root.account.data = initial_data;
    }
}

// Accounts anchor definition.

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[anchor(mut, "!root.initialized")]
    pub root: ProgramAccount<'info, Root>,
}

// Program owned account.

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct Root {
    pub initialized: bool,
    pub data: u64,
}
```

The program above does the following

* Transforms the accounts array into the `Initialize` struct.
* Performs all constraint checks. Here, ensuring that the `Root` account is not initialized
  by checking the *literal* constraint demarcated by double quotes "" and ensuring the `root`
  account is owned by the executing program.
* Saves the newly updated account state, marked as `mut`.

### Marking a program.

The `#[program]` attribute marks a program.

```rust
#[program]
mod example {
   ...
}
```

Internally, this generates the usual Solana entry code, i.e.,

```rust
solana_program::entrypoint!(entry);
fn entry(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    ...
}
```

Additionally, it will generate code to both 1) deserialize the `accounts` slice into a validated accounts, typed accounts struct, ensuring all specified constraints are satisified, and 2) deserialize the `instruction_data` and dispatch to the correct handler (e.g., `initialize` in the example above).

### Creating an instruction handler.

Each method inside the program corresponds to an instruction handler.

```rust
pub fn initialize(ctx: Context<Initialize>, initial_data: u64) {
  ...
}
```

Note that the `program` handler inputs are broken up into two sections: 1) a context containing an accounts struct for the instruction, deriving the `Accounts` macro, and a variable length set of program arguments deserialized from the instruction data.

## Marking an Accounts struct.

Account anchors are deserialized structs from the Solana `accounts` slice. To create one, mark your struct with the `#[derive(Accounts)]` macro.

```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[anchor(mut, "!root.initialized")]
    pub root: ProgramAccount<'info, Root>,
}
```

This anchor will perform constraint checks before your `initialize` instruction handler is called. This example, in particular, will execute the code *literal* provided `"!root.initialized"`. If any of the constraints fail to be satisfied, the instruction will exit with an error and your instruction handler will never be called. `mut` marks the account mutable and will be written to account storage on program exit.

## Accounts attribute syntax.

There are several inert attributes (attributes that are consumed only for the purposes of the Anchor macro) that can be specified on the struct deriving `Accounts`.

| Attribute | Where Applicable | Description |
|:--|:--|:--|
| `#[account(signer)]` | On raw `AccountInfo` structs. | Checks the given account signed the transaction. |
| `#[account(mut)]` | On `AnchorAccount` structs. | Marks the account as mutable and persists the state transition. |
| `#[account(belongs_to = <target>)]` | On `AnchorAccount` structs | Checks the `target` field on the account matches the `target` field in the accounts array. `target` name must match. |
| `#[account(owner = <program \| skip>)]` | On `AnchorAccount` and `AccountInfo` structs | Checks the owner of the account is the current program or skips the check. Defaults to `program`, if not given. |
| `#[account("<code-literal>")]` | On `AnchorAccount` structs | Executes the given code literal as a constraint. The literal should evaluate to a boolean. |
| `#[access_control(<method>)]` | On program instruction handlers | Runs `method` before executing the instruction. |

## Future work.

* Standalone constraint expressions. Define expressions in the same way you'd define any other type and then reference them from Anchor structs. This would allow sharing constraints between Anchor structs. Also could do something similar to solidity's function annotation.
* Constraints on containers. Accounts can be passed in as logical groups, e.g., `Vec<Root>` using the example above, or even as custom structs, e.g., `MyCustomContainer` (where each field itself is an instance of `ProgramAccount`), which might provide a more convient way to reference a group of accounts.
* Sysvars. Sysvars should be detected and auto deserialized with owner checks.
* SPL programs. Similarly, SPL programs should be detected and deserialized with owner checks.
* Client generation. It's straight forward to use the parsers here to emit an IDL that can be used to generate clients.
* Error code generation for each constraint.
* Relay accounts for composability
* Error code derive for boilerplate.
* Generate error codes for each constraint.

## License

Anchor is dual-licensed under Apache 2.0 and MIT terms:

```
Apache License, Version 2.0, (LICENSE or http://www.apache.org/licenses/LICENSE-2.0)
MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)
```
