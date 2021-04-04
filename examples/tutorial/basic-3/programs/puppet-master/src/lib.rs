// #region core
use anchor_lang::prelude::*;
use puppet::{Puppet, SetData};

#[program]
mod puppet_master {
    use super::*;
    pub fn pull_strings(ctx: Context<PullStrings>, data: u64) -> ProgramResult {
        let cpi_program = ctx.accounts.puppet_program.clone();
        let cpi_accounts = SetData {
            puppet: ctx.accounts.puppet.clone().into(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        puppet::cpi::set_data(cpi_ctx, data)
    }
}

#[derive(Accounts)]
pub struct PullStrings<'info> {
    #[account(mut)]
    pub puppet: CpiAccount<'info, Puppet>,
    pub puppet_program: AccountInfo<'info>,
}
// #endregion core
