use crate::error::ErrorCode;
use crate::{
    AccountDeserialize, AccountSerialize, Accounts, AccountsExit, CpiAccount, ToAccountInfo,
    ToAccountInfos, ToAccountMetas,
};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::ops::{Deref, DerefMut};

pub const PROGRAM_STATE_SEED: &'static str = "unversioned";

/// Boxed container for the program state singleton.
#[derive(Clone)]
pub struct ProgramState<'info, T: AccountSerialize + AccountDeserialize + Clone> {
    inner: Box<Inner<'info, T>>,
}

#[derive(Clone)]
struct Inner<'info, T: AccountSerialize + AccountDeserialize + Clone> {
    info: AccountInfo<'info>,
    account: T,
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> ProgramState<'a, T> {
    pub fn new(info: AccountInfo<'a>, account: T) -> ProgramState<'a, T> {
        Self {
            inner: Box::new(Inner { info, account }),
        }
    }

    /// Deserializes the given `info` into a `ProgramState`.
    #[inline(never)]
    pub fn try_from(info: &AccountInfo<'a>) -> Result<ProgramState<'a, T>, ProgramError> {
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

impl<'info, T> Accounts<'info> for ProgramState<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Clone,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
    ) -> Result<Self, ProgramError> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];

        if account.key != &Self::address(program_id) {
            solana_program::msg!("Invalid state address");
            return Err(ErrorCode::StateInvalidAddress.into());
        }

        let pa = ProgramState::try_from(account)?;
        if pa.inner.info.owner != program_id {
            solana_program::msg!("Invalid state owner");
            return Err(ErrorCode::AccountNotProgramOwned.into());
        }
        Ok(pa)
    }
}

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

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountInfos<'info>
    for ProgramState<'info, T>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.inner.info.clone()]
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountInfo<'info>
    for ProgramState<'info, T>
{
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.inner.info.clone()
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> Deref for ProgramState<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &(*self.inner).account
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> DerefMut for ProgramState<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut DerefMut::deref_mut(&mut self.inner).account
    }
}

impl<'info, T> From<CpiAccount<'info, T>> for ProgramState<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Clone,
{
    fn from(a: CpiAccount<'info, T>) -> Self {
        Self::new(a.to_account_info(), Deref::deref(&a).clone())
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AccountsExit<'info>
    for ProgramState<'info, T>
{
    fn exit(&self, _program_id: &Pubkey) -> ProgramResult {
        let info = self.to_account_info();
        let mut data = info.try_borrow_mut_data()?;
        let dst: &mut [u8] = &mut data;
        let mut cursor = std::io::Cursor::new(dst);
        self.inner.account.try_serialize(&mut cursor)?;
        Ok(())
    }
}

pub fn address(program_id: &Pubkey) -> Pubkey {
    let (base, _nonce) = Pubkey::find_program_address(&[], program_id);
    let seed = PROGRAM_STATE_SEED;
    let owner = program_id;
    Pubkey::create_with_seed(&base, seed, owner).unwrap()
}
