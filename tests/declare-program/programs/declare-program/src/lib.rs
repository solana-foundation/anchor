use anchor_lang::prelude::*;

declare_id!("Dec1areProgram11111111111111111111111111111");

declare_program!(external);
use external::program::External;

// Compilation check for legacy IDL (pre Anchor `0.30`)
declare_program!(external_legacy);

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

    pub fn event_utils(_ctx: Context<Utils>) -> Result<()> {
        use external::utils::Event;

        // Empty
        if Event::try_from_bytes(&[]).is_ok() {
            return Err(ProgramError::Custom(0).into());
        }

        const DISC: &[u8] =
            &<external::events::MyEvent as anchor_lang::Discriminator>::DISCRIMINATOR;

        // Correct discriminator but invalid data
        if Event::try_from_bytes(DISC).is_ok() {
            return Err(ProgramError::Custom(1).into());
        };

        // Correct discriminator and valid data
        match Event::try_from_bytes(&[DISC, &[1, 0, 0, 0]].concat()) {
            Ok(Event::MyEvent(my_event)) => require_eq!(my_event.value, 1),
            Err(e) => return Err(e.into()),
        }

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

#[derive(Accounts)]
pub struct Utils<'info> {
    pub authority: Signer<'info>,
}
