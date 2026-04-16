use anchor_lang_v2::prelude::*;

declare_id!("8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR");

#[program]
pub mod caller {
    use super::*;

    pub fn proxy_set_data(ctx: &mut Context<ProxySetData>, value: u64) -> Result<()> {
        let cpi_accounts = callee::cpi::accounts::SetData {
            data: ctx.accounts.callee_data.cpi_handle_mut(),
            authority: ctx.accounts.authority.cpi_handle(),
        };
        let cpi_ctx =
            CpiContext::new(ctx.accounts.callee_program.address(), cpi_accounts);
        callee::cpi::set_data(cpi_ctx, value)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct ProxySetData {
    #[account(mut)]
    pub callee_data: UncheckedAccount,
    pub authority: Signer,
    pub callee_program: UncheckedAccount,
}
