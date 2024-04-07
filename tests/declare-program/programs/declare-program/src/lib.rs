use anchor_lang::prelude::*;

declare_id!("Dec1areProgram11111111111111111111111111111");

declare_program!(external);
use external::program::External;

#[program]
pub mod declare_program {
    use super::*;

    pub fn cpi(ctx: Context<Cpi>, value: u32) -> Result<()> {
        let cpi_my_account = &mut ctx.accounts.cpi_my_account;
        require_keys_eq!(external::accounts::MyAccount::owner(), external::ID);
        require_eq!(cpi_my_account.field, 0);

        let cpi_ctx = CpiContext::new(
            ctx.accounts.external_program.to_account_info(),
            external::cpi::accounts::Update {
                authority: ctx.accounts.authority.to_account_info(),
                my_account: cpi_my_account.to_account_info(),
            },
        );
        external::cpi::update(cpi_ctx, value)?;

        cpi_my_account.reload()?;
        require_eq!(cpi_my_account.field, value);

        Ok(())
    }

    pub fn cpi_composite(ctx: Context<Cpi>, value: u32) -> Result<()> {
        let cpi_my_account = &mut ctx.accounts.cpi_my_account;

        let cpi_ctx = CpiContext::new(
            ctx.accounts.external_program.to_account_info(),
            external::cpi::accounts::UpdateComposite {
                update: external::cpi::accounts::Update {
                    authority: ctx.accounts.authority.to_account_info(),
                    my_account: cpi_my_account.to_account_info(),
                },
            },
        );
        external::cpi::update_composite(cpi_ctx, value)?;

        cpi_my_account.reload()?;
        require_eq!(cpi_my_account.field, value);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Cpi<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub cpi_my_account: Account<'info, external::accounts::MyAccount>,
    pub external_program: Program<'info, External>,
}
