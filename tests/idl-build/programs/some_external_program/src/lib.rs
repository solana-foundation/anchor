use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod idl_2 {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>, _baz: Baz) -> Result<()> {
        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Baz {
    some_field: u8,
}

#[derive(Accounts)]
pub struct Initialize {}
