pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;
pub use state::*;

declare_id!("CwrqeMj2U8tFr1Rhkgwc84tpAsqbt9pTt2a4taoTADPr");

#[program]
pub mod template_multiple_files {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }
}
