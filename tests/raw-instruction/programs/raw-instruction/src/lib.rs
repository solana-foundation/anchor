use anchor_lang::prelude::*;

declare_id!("HR8CJFL12vvXBMuP5FjamjNeRhPgkGv4f8DYUa93RdH8");

#[program]
pub mod raw_instruction {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, data: u64) -> Result<()> {
        ctx.accounts.data_account.data = data;
        msg!("Initialized with data: {}", data);
        Ok(())
    }

    /// Raw instruction that accepts &[u8] directly
    /// Must be explicitly marked with #[raw] attribute to skip instruction deserialization
    #[raw]
    pub fn raw_handler(ctx: Context<RawHandler>, data: &[u8]) -> Result<()> {
        msg!("Raw data length: {}", data.len());
        
        // Parse custom format from raw bytes
        if data.len() >= 8 {
            let value = u64::from_le_bytes([
                data[0], data[1], data[2], data[3],
                data[4], data[5], data[6], data[7],
            ]);
            ctx.accounts.data_account.data = value;
            msg!("Parsed value from raw bytes: {}", value);
        } else {
            return Err(ErrorCode::InvalidDataLength.into());
        }
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + 8)]
    pub data_account: Account<'info, DataAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RawHandler<'info> {
    #[account(mut)]
    pub data_account: Account<'info, DataAccount>,
}

#[account]
pub struct DataAccount {
    pub data: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid data length")]
    InvalidDataLength,
}
