use crate::error::ErrorCode;
use crate::{
    AccountDeserialize, AccountSerialize, Accounts, AccountsExit, Key, ToAccountInfo,
    ToAccountInfos, ToAccountMetas,
};
#[allow(deprecated)]
use crate::{CpiStateContext, ProgramState};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::ops::{Deref, DerefMut};

/// Boxed container for the program state singleton, used when the state
/// is for a program not currently executing.
#[derive(Clone)]
#[deprecated]
pub struct CpiState<'info, T: AccountSerialize + AccountDeserialize + Clone> {
    inner: Box<Inner<'info, T>>,
}

#[derive(Clone)]
struct Inner<'info, T: AccountSerialize + AccountDeserialize + Clone> {
    info: AccountInfo<'info>,
    account: T,
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> CpiState<'info, T> {
    pub fn new(i: AccountInfo<'info>, account: T) -> CpiState<'info, T> {
        Self {
            inner: Box::new(Inner { info: i, account }),
        }
    }

    /// Deserializes the given `info` into a `CpiState`.
    #[inline(never)]
    pub fn try_from(info: &AccountInfo<'info>) -> Result<CpiState<'info, T>, ProgramError> {
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(CpiState::new(info.clone(), T::try_deserialize(&mut data)?))
    }

    fn seed() -> &'static str {
        ProgramState::<T>::seed()
    }

    pub fn address(program_id: &Pubkey) -> Pubkey {
        let (base, _nonce) = Pubkey::find_program_address(&[], program_id);
        let seed = Self::seed();
        let owner = program_id;
        Pubkey::create_with_seed(&base, seed, owner).unwrap()
    }

    /// Convenience api for creating a `CpiStateContext`.
    pub fn context<'a, 'b, 'c, A: Accounts<'info>>(
        &self,
        program: AccountInfo<'info>,
        accounts: A,
    ) -> CpiStateContext<'a, 'b, 'c, 'info, A> {
        CpiStateContext::new(program, self.inner.info.clone(), accounts)
    }
}

#[allow(deprecated)]
impl<'info, T> Accounts<'info> for CpiState<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Clone,
{
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
    ) -> Result<Self, ProgramError> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];

        // No owner or address check is done here. One must use the
        // #[account(state = <account-name>)] constraint.

        CpiState::try_from(account)
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountMetas
    for CpiState<'info, T>
{
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.inner.info.is_signer);
        let meta = match self.inner.info.is_writable {
            false => AccountMeta::new_readonly(*self.inner.info.key, is_signer),
            true => AccountMeta::new(*self.inner.info.key, is_signer),
        };
        vec![meta]
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountInfos<'info>
    for CpiState<'info, T>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.inner.info.clone()]
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountInfo<'info>
    for CpiState<'info, T>
{
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.inner.info.clone()
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AsRef<AccountInfo<'info>>
    for CpiState<'info, T>
{
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.inner.info
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> Deref for CpiState<'info, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &(*self.inner).account
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> DerefMut for CpiState<'info, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut DerefMut::deref_mut(&mut self.inner).account
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AccountsExit<'info>
    for CpiState<'info, T>
{
    fn exit(&self, _program_id: &Pubkey) -> ProgramResult {
        // no-op
        Ok(())
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> Key for CpiState<'info, T> {
    fn key(&self) -> Pubkey {
        *self.inner.info.key
    }
}
