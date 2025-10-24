use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;

pub fn transfer<'info>(
    user_account: &mut Account<'info, UserAccount>,
    authority: &mut Signer<'info>,
    amount: u64,
) -> Result<()> {
    anchor_lang::solana_program::program::invoke(
        &system_instruction::transfer(&user_account.key(), &authority.key(), amount),
        &[user_account.to_account_info(), authority.to_account_info()],
    )?;
    Ok(())
}
