use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_lang::solana_program::program_error::ProgramError;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{Accounts, CpiContext};

pub use spl_associated_token_account::{get_associated_token_address, ID};

pub fn create<'info>(ctx: CpiContext<'_, '_, '_, 'info, Create<'info>>) -> ProgramResult {
    let ix = spl_associated_token_account::create_associated_token_account(
        ctx.accounts.payer.key,
        ctx.accounts.authority.key,
        ctx.accounts.mint.key,
    );
    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.payer,
            ctx.accounts.associated_token,
            ctx.accounts.authority,
            ctx.accounts.mint,
            ctx.accounts.system_program,
            ctx.accounts.token_program,
            ctx.accounts.rent,
        ],
        ctx.signer_seeds,
    )
}

#[derive(Accounts)]
pub struct Create<'info> {
    pub payer: AccountInfo<'info>,
    pub associated_token: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

#[derive(Clone)]
pub struct AssociatedToken;

impl anchor_lang::AccountDeserialize for AssociatedToken {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        AssociatedToken::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(_buf: &mut &[u8]) -> Result<Self, ProgramError> {
        Ok(AssociatedToken)
    }
}

impl anchor_lang::Id for AssociatedToken {
    fn id() -> Pubkey {
        ID
    }
}
