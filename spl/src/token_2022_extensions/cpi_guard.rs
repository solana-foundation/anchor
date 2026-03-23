// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::Result;
use anchor_lang::{context::CpiContext, Accounts};
use spl_token_2022_interface as spl_token_2022;

pub fn cpi_guard_enable<'info>(ctx: CpiContext<'_, '_, '_, 'info, CpiGuard<'info>>) -> Result<()> {
    let ix = spl_token_2022::extension::cpi_guard::instruction::enable_cpi_guard(
        ctx.accounts.token_program_id.key,
        ctx.accounts.account.key,
        ctx.accounts.owner.key,
        &[],
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.account,
            ctx.accounts.owner,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn cpi_guard_disable<'info>(ctx: CpiContext<'_, '_, '_, 'info, CpiGuard<'info>>) -> Result<()> {
    let ix = spl_token_2022::extension::cpi_guard::instruction::disable_cpi_guard(
        ctx.accounts.token_program_id.key,
        ctx.accounts.account.key,
        ctx.accounts.owner.key,
        &[],
    )?;

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.account,
            ctx.accounts.owner,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct CpiGuard<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub account: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
}

#[cfg(test)]
mod tests {
    use anchor_lang::solana_program::pubkey::Pubkey;
    use spl_token_2022_interface as spl_token_2022;

    /// Verifies enable_cpi_guard instruction uses the correct owner key (token authority),
    /// not account.owner (the program that owns the account data).
    ///
    /// The bug: the original code passed `ctx.accounts.account.owner` which is the
    /// program ID that owns the account, not the token account authority.
    #[test]
    fn test_enable_cpi_guard_correct_accounts() {
        let token_program_id = Pubkey::new_unique();
        let account_pubkey = Pubkey::new_unique();
        let owner_pubkey = Pubkey::new_unique();

        let ix = spl_token_2022::extension::cpi_guard::instruction::enable_cpi_guard(
            &token_program_id,
            &account_pubkey,
            &owner_pubkey,
            &[],
        )
        .expect("enable_cpi_guard instruction construction should succeed");

        assert_eq!(ix.accounts.len(), 3, "enable_cpi_guard expects 3 account metas: program, account, owner");
        assert_eq!(ix.accounts[1].pubkey, account_pubkey, "account meta at index 1 should be the token account");
        assert_eq!(ix.accounts[2].pubkey, owner_pubkey, "account meta at index 2 should be the owner/authority, not the program owner");
    }

    /// Verifies disable_cpi_guard has the same correct account structure.
    #[test]
    fn test_disable_cpi_guard_correct_accounts() {
        let token_program_id = Pubkey::new_unique();
        let account_pubkey = Pubkey::new_unique();
        let owner_pubkey = Pubkey::new_unique();

        let ix = spl_token_2022::extension::cpi_guard::instruction::disable_cpi_guard(
            &token_program_id,
            &account_pubkey,
            &owner_pubkey,
            &[],
        )
        .expect("disable_cpi_guard instruction construction should succeed");

        assert_eq!(ix.accounts.len(), 3);
        assert_eq!(ix.accounts[1].pubkey, account_pubkey);
        assert_eq!(ix.accounts[2].pubkey, owner_pubkey);
    }
}
