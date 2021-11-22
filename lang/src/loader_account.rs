use crate::error::ErrorCode;
use crate::{impl_account_info_traits, impl_account_metas_trait, impl_accounts_trait};
use crate::{
    Accounts, AccountsClose, AccountsExit, Key, Owner, ToAccountInfo, ToAccountInfos,
    ToAccountMetas, ZeroCopy,
};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::cell::{Ref, RefMut};
use std::fmt;
use std::io::Write;
use std::marker::PhantomData;
use std::ops::DerefMut;

/// Account AccountLoader facilitating on demand zero copy deserialization.
/// Note that using accounts in this way is distinctly different from using,
/// for example, the [`ProgramAccount`](./struct.ProgramAccount.html). Namely,
/// one must call `load`, `load_mut`, or `load_init`, before reading or writing
/// to the account. For more details on zero-copy-deserialization, see the
/// [`account`](./attr.account.html) attribute.
///
/// When using it's important to be mindful of any calls to `load` so as not to
/// induce a `RefCell` panic, especially when sharing accounts across CPI
/// boundaries. When in doubt, one should make sure all refs resulting from a
/// call to `load` are dropped before CPI.
#[derive(Clone)]
pub struct AccountLoader<'info, T: ZeroCopy + Owner> {
    info: AccountInfo<'info>,
    phantom: PhantomData<&'info T>,
}

impl<'info, T: ZeroCopy + Owner + fmt::Debug> fmt::Debug for AccountLoader<'info, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountLoader")
            .field("info", &self.info)
            .field("phantom", &self.phantom)
            .finish()
    }
}

impl<'info, T: ZeroCopy + Owner> AccountLoader<'info, T> {
    fn new(info: AccountInfo<'info>) -> AccountLoader<'info, T> {
        Self {
            info,
            phantom: PhantomData,
        }
    }

    /// Constructs a new `Loader` from a previously initialized account.
    #[inline(never)]
    pub fn try_from(info: &AccountInfo<'info>) -> Result<AccountLoader<'info, T>, ProgramError> {
        if info.owner != &T::owner() {
            return Err(ErrorCode::AccountNotProgramOwned.into());
        }
        let data: &[u8] = &info.try_borrow_data()?;
        // Discriminator must match.
        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        if disc_bytes != T::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(AccountLoader::new(info.clone()))
    }

    /// Constructs a new `Loader` from an uninitialized account.
    #[inline(never)]
    pub fn try_from_unchecked(
        _program_id: &Pubkey,
        info: &AccountInfo<'info>,
    ) -> Result<AccountLoader<'info, T>, ProgramError> {
        if info.owner != &T::owner() {
            return Err(ErrorCode::AccountNotProgramOwned.into());
        }
        Ok(AccountLoader::new(info.clone()))
    }

    /// Returns a Ref to the account data structure for reading.
    pub fn load(&self) -> Result<Ref<T>, ProgramError> {
        let data = self.info.try_borrow_data()?;

        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        if disc_bytes != T::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(Ref::map(data, |data| bytemuck::from_bytes(&data[8..])))
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    pub fn load_mut(&self) -> Result<RefMut<T>, ProgramError> {
        // AccountInfo api allows you to borrow mut even if the account isn't
        // writable, so add this check for a better dev experience.
        if !self.info.is_writable {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let data = self.info.try_borrow_mut_data()?;

        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        if disc_bytes != T::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(RefMut::map(data, |data| {
            bytemuck::from_bytes_mut(&mut data.deref_mut()[8..])
        }))
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    /// Should only be called once, when the account is being initialized.
    pub fn load_init(&self) -> Result<RefMut<T>, ProgramError> {
        // AccountInfo api allows you to borrow mut even if the account isn't
        // writable, so add this check for a better dev experience.
        if !self.info.is_writable {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let data = self.info.try_borrow_mut_data()?;

        // The discriminator should be zero, since we're initializing.
        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        let discriminator = u64::from_le_bytes(disc_bytes);
        if discriminator != 0 {
            return Err(ErrorCode::AccountDiscriminatorAlreadySet.into());
        }

        Ok(RefMut::map(data, |data| {
            bytemuck::from_bytes_mut(&mut data.deref_mut()[8..])
        }))
    }
}

impl<'info, T: ZeroCopy + Owner> AccountsExit<'info> for AccountLoader<'info, T> {
    // The account *cannot* be loaded when this is called.
    fn exit(&self, _program_id: &Pubkey) -> ProgramResult {
        let mut data = self.info.try_borrow_mut_data()?;
        let dst: &mut [u8] = &mut data;
        let mut cursor = std::io::Cursor::new(dst);
        cursor.write_all(&T::discriminator()).unwrap();
        Ok(())
    }
}

impl<'info, T: ZeroCopy + Owner> AccountsClose<'info> for AccountLoader<'info, T> {
    fn close(&self, sol_destination: AccountInfo<'info>) -> ProgramResult {
        crate::common::close(self.to_account_info(), sol_destination)
    }
}

impl_account_info_traits!(AccountLoader<'info, T> where T: ZeroCopy + Owner);
impl_accounts_trait!(AccountLoader<'info, T> where T: ZeroCopy + Owner);
impl_account_metas_trait!(AccountLoader<'info, T> where T: ZeroCopy + Owner);
