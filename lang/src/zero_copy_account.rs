use crate::{
    Accounts, AccountsExit, AccountsInit, ToAccountInfo, ToAccountInfos, ToAccountMetas, ZeroCopy,
};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::cell::{Ref, RefMut};
use std::io::Write;
use std::marker::PhantomData;
use std::ops::DerefMut;

/// Account container facilitating on demand zero copy deserialization.
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
pub struct AccountLoader<'info, T: ZeroCopy> {
    acc_info: AccountInfo<'info>,
    phantom: PhantomData<&'info T>,
}

impl<'info, T: ZeroCopy> AccountLoader<'info, T> {
    fn new(acc_info: AccountInfo<'info>) -> AccountLoader<'info, T> {
        Self {
            acc_info,
            phantom: PhantomData,
        }
    }

    /// Constructs a new `AccountLoader` from a previously initialized account.
    #[inline(never)]
    pub fn try_from(
        acc_info: &AccountInfo<'info>,
    ) -> Result<AccountLoader<'info, T>, ProgramError> {
        let data: &[u8] = &acc_info.try_borrow_data()?;

        // Discriminator must match.
        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        if disc_bytes != T::discriminator() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(AccountLoader::new(acc_info.clone()))
    }

    /// Constructs a new `AccountLoader` from an uninitialized account.
    #[inline(never)]
    pub fn try_from_init(
        acc_info: &AccountInfo<'info>,
    ) -> Result<AccountLoader<'info, T>, ProgramError> {
        let data = acc_info.try_borrow_data()?;

        // The discriminator should be zero, since we're initializing.
        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        let discriminator = u64::from_le_bytes(disc_bytes);
        if discriminator != 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(AccountLoader::new(acc_info.clone()))
    }

    /// Returns a Ref to the account data structure for reading.
    pub fn load(&self) -> Result<Ref<T>, ProgramError> {
        let data = self.acc_info.try_borrow_data()?;

        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        if disc_bytes != T::discriminator() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Ref::map(data, |data| bytemuck::from_bytes(&data[8..])))
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    pub fn load_mut(&self) -> Result<RefMut<T>, ProgramError> {
        // AccountInfo api allows you to borrow mut even if the account isn't
        // writable, so add this check for a better dev experience.
        if !self.acc_info.is_writable {
            return Err(ProgramError::Custom(87)); // todo: proper error
        }

        let data = self.acc_info.try_borrow_mut_data()?;

        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        if disc_bytes != T::discriminator() {
            return Err(ProgramError::InvalidAccountData);
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
        if !self.acc_info.is_writable {
            return Err(ProgramError::Custom(87)); // todo: proper error
        }

        let data = self.acc_info.try_borrow_mut_data()?;

        // The discriminator should be zero, since we're initializing.
        let mut disc_bytes = [0u8; 8];
        disc_bytes.copy_from_slice(&data[..8]);
        let discriminator = u64::from_le_bytes(disc_bytes);
        if discriminator != 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(RefMut::map(data, |data| {
            bytemuck::from_bytes_mut(&mut data.deref_mut()[8..])
        }))
    }
}

impl<'info, T: ZeroCopy> Accounts<'info> for AccountLoader<'info, T> {
    #[inline(never)]
    fn try_accounts(
        program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
    ) -> Result<Self, ProgramError> {
        if accounts.is_empty() {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        let pa = AccountLoader::try_from(account)?;
        if pa.acc_info.owner != program_id {
            return Err(ProgramError::Custom(1)); // todo: proper error
        }
        Ok(pa)
    }
}

impl<'info, T: ZeroCopy> AccountsInit<'info> for AccountLoader<'info, T> {
    #[inline(never)]
    fn try_accounts_init(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
    ) -> Result<Self, ProgramError> {
        if accounts.is_empty() {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        AccountLoader::try_from_init(account)
    }
}

impl<'info, T: ZeroCopy> AccountsExit<'info> for AccountLoader<'info, T> {
    // The account *cannot* be loaded when this is called.
    fn exit(&self, _program_id: &Pubkey) -> ProgramResult {
        let mut data = self.acc_info.try_borrow_mut_data()?;
        let dst: &mut [u8] = &mut data;
        let mut cursor = std::io::Cursor::new(dst);
        cursor.write_all(&T::discriminator()).unwrap();
        Ok(())
    }
}

impl<'info, T: ZeroCopy> ToAccountMetas for AccountLoader<'info, T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.acc_info.is_signer);
        let meta = match self.acc_info.is_writable {
            false => AccountMeta::new_readonly(*self.acc_info.key, is_signer),
            true => AccountMeta::new(*self.acc_info.key, is_signer),
        };
        vec![meta]
    }
}

impl<'info, T: ZeroCopy> ToAccountInfos<'info> for AccountLoader<'info, T> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.acc_info.clone()]
    }
}

impl<'info, T: ZeroCopy> ToAccountInfo<'info> for AccountLoader<'info, T> {
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.acc_info.clone()
    }
}
