//! This example enforces the missing documentation lint.
#![deny(missing_docs)]

use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

/// Program for testing that the `missing_docs` lint can be applied.
#[program]
mod docs {
    use super::*;

    /// Hello.
    pub fn hello(_ctx: Context<Hello>) -> Result<()> {
        err!(MyError::Hello)
    }
}

/// Hello accounts.
#[derive(Accounts)]
pub struct Hello<'info> {
    /// Rent sysvar.
    /// Multi line docs.
    pub rent: Sysvar<'info, Rent>,
    /// Composite accounts test.
    /// Multiple lines supported.
    /// You can also include "double quotes".
    pub other: HelloComposite<'info>,
}

/// Hello accounts.
#[derive(Accounts)]
pub struct HelloComposite<'info> {
    /// Rent sysvar 2.
    pub rent2: Sysvar<'info, Rent>,
}

/// MyError.
#[error_code]
pub enum MyError {
    /// test
    #[msg("This is an error message clients will automatically display")]
    Hello,
    /// test2
    HelloNoMsg = 123,
    /// test3
    HelloNext,
    /// test4
    HelloCustom,
}
