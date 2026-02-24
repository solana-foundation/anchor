use anchor_lang::prelude::*;
use callee::cpi::accounts::Realloc;
use callee::program::Callee;

declare_id!("HmbTLCmaGvZhKnn1Zfa1JVnp7vkMV4DYVxPLWBVoN65L");

#[program]
pub mod caller {
    use super::*;

    pub fn call_realloc(ctx: Context<CallRealloc>, len: u16) -> Result<()> {
        let cpi_program = ctx.accounts.callee_program.key();
        let cpi_accounts = Realloc {
            authority: ctx.accounts.authority.to_account_info(),
            data_account: ctx.accounts.data_account.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        callee::cpi::realloc(cpi_ctx, len)
    }
}

#[derive(Accounts)]
pub struct CallRealloc<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: Validated by callee program
    #[account(mut)]
    pub data_account: UncheckedAccount<'info>,

    pub callee_program: Program<'info, Callee>,
    pub system_program: Program<'info, System>,
}
