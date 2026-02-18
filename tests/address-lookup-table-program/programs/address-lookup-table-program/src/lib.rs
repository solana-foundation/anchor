use anchor_lang::prelude::*;

use crate::program::AddressLookupTableTest;

declare_id!("Cum9tTyj5HwcEiAmhgaS7Bbj4UczCwsucrCkxRECzM4e");

#[program]
pub mod address_lookup_table_test {
    use super::*;

    pub fn test_read(
        ctx: Context<Test>,
    ) -> Result<()> {
        Ok(())
    }
}

#[error_code]
pub enum CustomError {
    InvalidProgramDataAddress,
    AccountNotProgram,
}

#[derive(Accounts)]
pub struct Test<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    pub table: Account<'info, AddressLookupTable>,
    pub lut_program: Program<'info, AddressLookupTableProgram>,
}
