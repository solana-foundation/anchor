use crate::{Accounts, ToAccountInfos, ToAccountMetas};
use solana_program::account_info::AccountInfo;
use solana_program::pubkey::Pubkey;

/// Provides non-argument inputs to the program.
pub struct Context<'a, 'b, 'c, 'info, T> {
    /// Currently executing program id.
    pub program_id: &'a Pubkey,
    /// Deserialized accounts.
    pub accounts: &'b mut T,
    /// Remaining accounts given but not deserialized or validated.
    pub remaining_accounts: &'c [AccountInfo<'info>],
}

impl<'a, 'b, 'c, 'info, T: Accounts<'info>> Context<'a, 'b, 'c, 'info, T> {
    pub fn new(
        program_id: &'a Pubkey,
        accounts: &'b mut T,
        remaining_accounts: &'c [AccountInfo<'info>],
    ) -> Self {
        Self {
            accounts,
            program_id,
            remaining_accounts,
        }
    }
}

/// Context speciying non-argument inputs for cross-program-invocations.
pub struct CpiContext<'a, 'b, 'c, 'info, T>
where
    T: ToAccountMetas + ToAccountInfos<'info>,
{
    pub accounts: T,
    pub program: AccountInfo<'info>,
    pub signer_seeds: &'a [&'b [&'c [u8]]],
}

impl<'a, 'b, 'c, 'info, T> CpiContext<'a, 'b, 'c, 'info, T>
where
    T: ToAccountMetas + ToAccountInfos<'info>,
{
    pub fn new(program: AccountInfo<'info>, accounts: T) -> Self {
        Self {
            accounts,
            program,
            signer_seeds: &[],
        }
    }

    pub fn new_with_signer(
        program: AccountInfo<'info>,
        accounts: T,
        signer_seeds: &'a [&'b [&'c [u8]]],
    ) -> Self {
        Self {
            accounts,
            program,
            signer_seeds,
        }
    }

    pub fn with_signer(mut self, signer_seeds: &'a [&'b [&'c [u8]]]) -> Self {
        self.signer_seeds = signer_seeds;
        self
    }
}
