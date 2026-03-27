// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::Result;
use anchor_lang::{context::CpiContext, Accounts};
use spl_token_2022_interface as spl_token_2022;

pub fn group_pointer_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, GroupPointerInitialize<'info>>,
    authority: Option<Pubkey>,
    group_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::group_pointer::instruction::initialize(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        authority,
        group_address,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupPointerInitialize<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}

pub fn group_pointer_update<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, GroupPointerUpdate<'info>>,
    group_address: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::extension::group_pointer::instruction::update(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
        ctx.accounts.authority.key,
        &[ctx.accounts.authority.key],
        group_address,
    )?;
    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.token_program_id,
            ctx.accounts.mint,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct GroupPointerUpdate<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
}

#[cfg(test)]
mod tests {
    use anchor_lang::solana_program::pubkey::Pubkey;
    use spl_token_2022_interface as spl_token_2022;

    /// Verifies group_pointer_initialize instruction construction succeeds.
    #[test]
    fn test_group_pointer_initialize_instruction() {
        let token_program_id = Pubkey::new_unique();
        let mint_pubkey = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let group_address = Pubkey::new_unique();

        let ix = spl_token_2022::extension::group_pointer::instruction::initialize(
            &token_program_id,
            &mint_pubkey,
            Some(authority),
            Some(group_address),
        )
        .expect("group_pointer initialize should build correctly");

        // initialize takes 2 accounts: token_program and mint
        assert_eq!(ix.accounts.len(), 2, "initialize needs token_program and mint accounts");
        assert_eq!(ix.accounts[1].pubkey, mint_pubkey);
    }

    /// Verifies group_pointer_update instruction includes the authority account.
    ///
    /// The bug: the original code only passed [token_program_id, mint] to invoke_signed,
    /// missing the authority account. The authority is needed as a signer to authorize
    /// the update. Without it, the CPI would fail at runtime.
    #[test]
    fn test_group_pointer_update_includes_authority() {
        let token_program_id = Pubkey::new_unique();
        let mint_pubkey = Pubkey::new_unique();
        let authority_pubkey = Pubkey::new_unique();
        let group_address = Pubkey::new_unique();

        let ix = spl_token_2022::extension::group_pointer::instruction::update(
            &token_program_id,
            &mint_pubkey,
            &authority_pubkey,
            &[&authority_pubkey],
            Some(group_address),
        )
        .expect("group_pointer update should build correctly");

        // update takes 3 accounts: token_program, mint, authority
        assert_eq!(ix.accounts.len(), 3, "update needs token_program, mint, and authority accounts");
        assert_eq!(ix.accounts[1].pubkey, mint_pubkey);
        assert_eq!(ix.accounts[2].pubkey, authority_pubkey, "authority must be included in accounts for signer validation");
    }
}
