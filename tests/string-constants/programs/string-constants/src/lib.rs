use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

/// String constant for testing IDL generation
#[constant]
pub const STRING_CONST: &str = "test string";

/// String constant with quotes
#[constant]
pub const STRING_WITH_QUOTES: &str = "test \"quoted\" string";

/// String constant with escape sequences
#[constant]
pub const STRING_WITH_ESCAPES: &str = "test\nstring\twith\rescapes";

/// Numeric constant for backward compatibility testing
#[constant]
pub const NUMBER_CONST: u64 = 12345;

/// Byte constant for backward compatibility testing
#[constant]
pub const BYTES_CONST: &[u8] = b"test bytes";

#[program]
pub mod string_constants {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {} 