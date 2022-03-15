#[allow(deprecated)]
use crate::accounts::cpi_account::CpiAccount;
use crate::bpf_writer::BpfWriter;
use crate::error::{Error, ErrorCode};
use crate::{
    AccountDeserialize, AccountSerialize, Accounts, AccountsClose, AccountsExit, Key, Result,
    ToAccountInfo, ToAccountInfos, ToAccountMetas,
};
use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

/// Boxed container for a deserialized `account`. Use this to reference any
/// account owned by the currently executing program.
#[derive(Clone)]
#[deprecated(since = "0.15.0", note = "Please use Account instead")]
pub struct ProgramAccount<'info, T: AccountSerialize + AccountDeserialize + Clone> {
    inner: Box<Inner<'info, T>>,
}

#[derive(Clone)]
struct Inner<'info, T: AccountSerialize + AccountDeserialize + Clone> {
    info: AccountInfo<'info>,
    account: T,
}

#[allow(deprecated)]
impl<'a, T: AccountSerialize + AccountDeserialize + Clone> ProgramAccount<'a, T> {
    fn new(info: AccountInfo<'a>, account: T) -> ProgramAccount<'a, T> {
        Self {
            inner: Box::new(Inner { info, account }),
        }
    }

    /// Deserializes the given `info` into a `ProgramAccount`.
    #[inline(never)]
    pub fn try_from(program_id: &Pubkey, info: &AccountInfo<'a>) -> Result<ProgramAccount<'a, T>> {
        if info.owner != program_id {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*info.owner, *program_id)));
        }
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(ProgramAccount::new(
            info.clone(),
            T::try_deserialize(&mut data)?,
        ))
    }

    /// Deserializes the given `info` into a `ProgramAccount` without checking
    /// the account discriminator. Be careful when using this and avoid it if
    /// possible.
    #[inline(never)]
    pub fn try_from_unchecked(
        program_id: &Pubkey,
        info: &AccountInfo<'a>,
    ) -> Result<ProgramAccount<'a, T>> {
        if info.owner != program_id {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*info.owner, *program_id)));
        }
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(ProgramAccount::new(
            info.clone(),
            T::try_deserialize_unchecked(&mut data)?,
        ))
    }

    pub fn into_inner(self) -> T {
        self.inner.account
    }
}

#[allow(deprecated)]
impl<'info, T> Accounts<'info> for ProgramAccount<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Clone,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut BTreeMap<String, u8>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        ProgramAccount::try_from(program_id, account)
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AccountsExit<'info>
    for ProgramAccount<'info, T>
{
    fn exit(&self, _program_id: &Pubkey) -> Result<()> {
        let info = self.to_account_info();
        let mut data = info.try_borrow_mut_data()?;
        let dst: &mut [u8] = &mut data;
        let mut writer = BpfWriter::new(dst);
        self.inner.account.try_serialize(&mut writer)?;
        Ok(())
    }
}

/// This function is for INTERNAL USE ONLY.
/// Do NOT use this function in a program.
/// Manual closing of `ProgramAccount<'info, T>` types is NOT supported.
///
/// Details: Using `close` with `ProgramAccount<'info, T>` is not safe because
/// it requires the `mut` constraint but for that type the constraint
/// overwrites the "closed account" discriminator at the end of the instruction.
#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AccountsClose<'info>
    for ProgramAccount<'info, T>
{
    fn close(&self, sol_destination: AccountInfo<'info>) -> Result<()> {
        crate::common::close(self.to_account_info(), sol_destination)
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountMetas
    for ProgramAccount<'info, T>
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
    for ProgramAccount<'info, T>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.inner.info.clone()]
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AsRef<AccountInfo<'info>>
    for ProgramAccount<'info, T>
{
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.inner.info
    }
}

#[allow(deprecated)]
impl<'a, T: AccountSerialize + AccountDeserialize + Clone> Deref for ProgramAccount<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &(*self.inner).account
    }
}

#[allow(deprecated)]
impl<'a, T: AccountSerialize + AccountDeserialize + Clone> DerefMut for ProgramAccount<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        #[cfg(feature = "anchor-debug")]
        if !self.inner.info.is_writable {
            solana_program::msg!("The given ProgramAccount is not mutable");
            panic!();
        }

        &mut DerefMut::deref_mut(&mut self.inner).account
    }
}

#[allow(deprecated)]
impl<'info, T> From<CpiAccount<'info, T>> for ProgramAccount<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Clone,
{
    fn from(a: CpiAccount<'info, T>) -> Self {
        Self::new(a.to_account_info(), Deref::deref(&a).clone())
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> Key for ProgramAccount<'info, T> {
    fn key(&self) -> Pubkey {
        *self.inner.info.key
    }
}
