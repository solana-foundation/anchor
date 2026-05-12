use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkgqUQBoiQphr");

mod events;
mod helper; // contains `+ use<T>` syntax that syn 1.x cannot parse
pub use events::*;

#[program]
pub mod idl_type_alias {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
