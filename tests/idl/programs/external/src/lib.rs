use anchor_lang::prelude::*;

declare_id!("Externa1111111111111111111111111111111111111");

#[program]
pub mod external {
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
