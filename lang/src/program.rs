use crate::error::ErrorCode;
use crate::*;
use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::fmt;
use std::ops::Deref;

/// Account container that checks ownership on deserialization.
#[derive(Clone)]
pub struct Program<'info, T: Id + AccountDeserialize + Clone> {
    _account: T,
    info: AccountInfo<'info>,
}

impl<'info, T: Id + AccountDeserialize + Clone + fmt::Debug> fmt::Debug for Program<'info, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Program")
            .field("account", &self._account)
            .field("info", &self.info)
            .finish()
    }
}

impl<'a, T: Id + AccountDeserialize + Clone> Program<'a, T> {
    fn new(info: AccountInfo<'a>, _account: T) -> Program<'a, T> {
        Self { info, _account }
    }

    /// Deserializes the given `info` into a `Program`.
    #[inline(never)]
    pub fn try_from(info: &AccountInfo<'a>) -> Result<Program<'a, T>, ProgramError> {
        if info.key != &T::id() {
            return Err(ErrorCode::InvalidProgramId.into());
        }
        if !info.executable {
            return Err(ErrorCode::InvalidProgramExecutable.into());
        }
        // Programs have no data so use an empty slice.
        let mut empty = &[][..];
        Ok(Program::new(info.clone(), T::try_deserialize(&mut empty)?))
    }
}

impl<'info, T: Id + AccountDeserialize + Clone> Deref for Program<'info, T> {
    type Target = AccountInfo<'info>;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl<'info, T: AccountDeserialize + Id + Clone> AccountsExit<'info> for Program<'info, T> {}

impl_account_info_traits!(Program<'info, T> where T: AccountDeserialize + Id + Clone);
impl_accounts_trait!(Program<'info, T> where T: AccountDeserialize + Id + Clone);
impl_account_metas_trait!(Program<'info, T> where T: AccountDeserialize + Id + Clone);
