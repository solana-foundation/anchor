#[allow(deprecated)]
use crate::accounts::cpi_account::CpiAccount;
use crate::bpf_writer::BpfWriter;
use crate::error::{Error, ErrorCode};
use crate::{
    AccountDeserialize, AccountSerialize, Accounts, AccountsExit, Key, Result, ToAccountInfo,
    ToAccountInfos, ToAccountMetas,
};
use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

pub const PROGRAM_STATE_SEED: &str = "unversioned";

/// Boxed container for the program state singleton.
#[derive(Clone)]
#[deprecated]
pub struct ProgramState<'info, T: AccountSerialize + AccountDeserialize + Clone> {
    inner: Box<Inner<'info, T>>,
}

#[derive(Clone)]
struct Inner<'info, T: AccountSerialize + AccountDeserialize + Clone> {
    info: AccountInfo<'info>,
    account: T,
}

#[allow(deprecated)]

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> ProgramState<'a, T> {
    fn new(info: AccountInfo<'a>, account: T) -> ProgramState<'a, T> {
        Self {
            inner: Box::new(Inner { info, account }),
        }
    }

    /// Deserializes the given `info` into a `ProgramState`.
    #[inline(never)]
    pub fn try_from(program_id: &Pubkey, info: &AccountInfo<'a>) -> Result<ProgramState<'a, T>> {
        if info.owner != program_id {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*info.owner, *program_id)));
        }
        if info.key != &Self::address(program_id) {
            solana_program::msg!("Invalid state address");
            return Err(ErrorCode::StateInvalidAddress.into());
        }
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(ProgramState::new(
            info.clone(),
            T::try_deserialize(&mut data)?,
        ))
    }

    pub fn seed() -> &'static str {
        PROGRAM_STATE_SEED
    }

    pub fn address(program_id: &Pubkey) -> Pubkey {
        address(program_id)
    }
}

#[allow(deprecated)]
impl<'info, T> Accounts<'info> for ProgramState<'info, T>
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
        ProgramState::try_from(program_id, account)
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountMetas
    for ProgramState<'info, T>
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
    for ProgramState<'info, T>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.inner.info.clone()]
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AsRef<AccountInfo<'info>>
    for ProgramState<'info, T>
{
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.inner.info
    }
}

#[allow(deprecated)]
impl<'a, T: AccountSerialize + AccountDeserialize + Clone> Deref for ProgramState<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &(*self.inner).account
    }
}

#[allow(deprecated)]
impl<'a, T: AccountSerialize + AccountDeserialize + Clone> DerefMut for ProgramState<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut DerefMut::deref_mut(&mut self.inner).account
    }
}

#[allow(deprecated)]
impl<'info, T> From<CpiAccount<'info, T>> for ProgramState<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Clone,
{
    fn from(a: CpiAccount<'info, T>) -> Self {
        Self::new(a.to_account_info(), Deref::deref(&a).clone())
    }
}

#[allow(deprecated)]
impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AccountsExit<'info>
    for ProgramState<'info, T>
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

pub fn address(program_id: &Pubkey) -> Pubkey {
    let (base, _nonce) = Pubkey::find_program_address(&[], program_id);
    let seed = PROGRAM_STATE_SEED;
    let owner = program_id;
    Pubkey::create_with_seed(&base, seed, owner).unwrap()
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> Key for ProgramState<'info, T> {
    fn key(&self) -> Pubkey {
        *self.inner.info.key
    }
}
