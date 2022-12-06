// This file is autogenerated with https://github.com/acheroncrypto/native-to-anchor

use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111111");

#[program]
pub mod spl_stateless_asks {
    use super::*;

    pub fn accept_offer(
        ctx: Context<AcceptOffer>,
        has_metadata: bool,
        maker_size: u64,
        taker_size: u64,
        bump_seed: u8,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct AcceptOffer<'info> {
    maker_wallet: AccountInfo<'info>,
    taker_wallet: Signer<'info>,
    #[account(mut)]
    maker_src_account: AccountInfo<'info>,
    #[account(mut)]
    maker_dst_account: AccountInfo<'info>,
    #[account(mut)]
    taker_src_account: AccountInfo<'info>,
    #[account(mut)]
    taker_dst_account: AccountInfo<'info>,
    maker_mint: AccountInfo<'info>,
    taker_mint: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    token_program: Program<'info, Token>,
    // optional_system_program: Program<'info, System>,
}

#[error_code]
pub enum UtilError {
    #[msg("PublicKeyMismatch")]
    PublicKeyMismatch,
    #[msg("InvalidMintAuthority")]
    InvalidMintAuthority,
    #[msg("UninitializedAccount")]
    UninitializedAccount,
    #[msg("IncorrectOwner")]
    IncorrectOwner,
    #[msg("PublicKeysShouldBeUnique")]
    PublicKeysShouldBeUnique,
    #[msg("StatementFalse")]
    StatementFalse,
    #[msg("NotRentExempt")]
    NotRentExempt,
    #[msg("NumericalOverflow")]
    NumericalOverflow,
}
