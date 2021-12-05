use anchor_lang::prelude::*;

declare_id!("Cum9tTyj5HwcEiAmhgaS7Bbj4UczCwsucrCkxRECzM4e");

// TODO: Once anchor can deserialize data of programs (=programdata_address) automatically, add another test to this file.
// Instead of using UpgradeableLoaderState, it should use Program<'info, MY_PROGRAM>

#[program]
pub mod bpf_upgradeable_state {
    use super::*;
    pub fn set_admin_settings(ctx: Context<SetAdminSettings>, admin_data: u64) -> ProgramResult {
        match *ctx.accounts.program {
            UpgradeableLoaderState::Program {
                programdata_address,
            } => {
                if programdata_address != ctx.accounts.program_data.key() {
                    return Err(CustomError::InvalidProgramDataAddress.into());
                }
            }
            _ => {
                return Err(CustomError::AccountNotProgram.into());
            }
        };
        ctx.accounts.settings.admin_data = admin_data;
        Ok(())
    }
}

#[account]
#[derive(Default, Debug)]
pub struct Settings {
    admin_data: u64,
}

#[error]
pub enum CustomError {
    InvalidProgramDataAddress,
    AccountNotProgram,
}

#[derive(Accounts)]
#[instruction(admin_data: u64)]
pub struct SetAdminSettings<'info> {
    #[account(init, payer = authority)]
    pub settings: Account<'info, Settings>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(address = crate::ID)]
    pub program: Account<'info, UpgradeableLoaderState>,
    #[account(constraint = program_data.upgrade_authority_address == Some(authority.key()))]
    pub program_data: Account<'info, ProgramData>,
    pub system_program: Program<'info, System>,
}
