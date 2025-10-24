use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;

/// Helper function for CPI - properly handles account passing
pub fn do_transfer<'info>(
    from: &Account<'info, crate::state::UserAccount>,
    to: &Signer<'info>,
    amount: u64,
) -> Result<()> {
    anchor_lang::solana_program::program::invoke(
        &system_instruction::transfer(&from.key(), &to.key(), amount),
        &[from.to_account_info(), to.to_account_info()],
    )?;
    Ok(())
}
