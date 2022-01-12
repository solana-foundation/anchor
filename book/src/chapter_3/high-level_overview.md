# High-level Overview
An Anchor program consists of three parts. The `program` module, the Accounts structs which are marked with `#[derive(Accounts)]`, and the `declareId` macro. The `program` module is where you write your business logic. The Accounts structs is where you validate accounts. The`declareId` macro creates an `ID` field that stores the address of your program.

When you start up a new Anchor project, you'll see the following:
```rust,ignore
// use this import to gain access to common anchor features
use anchor_lang::prelude::*;

// declare an id for your program
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

// write your business logic here
#[program]
mod hello_anchor {
    use super::*;
    pub fn initialize(_ctx: Context<Initialize>) -> ProgramResult {
        Ok(())
    }
}

// validate incoming accounts here
#[derive(Accounts)]
pub struct Initialize {}
```

We'll go into more detail in the next sections but for now, note that the way an endpoint is connected to its corresponding Accounts struct is the `ctx` argument in the endpoint. The argument is of type `Context` which is generic over an Accounts struct, i.e. this is where you put the name of your account validation struct. In this example, it's `Initialize`.