use crate::bpf_writer::BpfWriter;
use crate::error::{Error, ErrorCode};
use crate::{
    Accounts, AccountsClose, AccountsExit, Key, Result, ToAccountInfo, ToAccountInfos,
    ToAccountMetas, ZeroCopy,
};
use arrayref::array_ref;
use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use std::cell::{Ref, RefMut};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Write;
use std::marker::PhantomData;
use std::ops::DerefMut;

/// Account loader facilitating on demand zero copy deserialization.
/// Note that using accounts in this way is distinctly different from using,
/// for example, the [`Account`](./struct.Account.html). Namely,
/// one must call `load`, `load_mut`, or `load_init`, before reading or writing
/// to the account. For more details on zero-copy-deserialization, see the
/// [`account`](./attr.account.html) attribute.
///
/// When using it's important to be mindful of any calls to `load` so as not to
/// induce a `RefCell` panic, especially when sharing accounts across CPI
/// boundaries. When in doubt, one should make sure all refs resulting from a
/// call to `load` are dropped before CPI.
#[deprecated(since = "0.18.0", note = "Please use AccountLoader instead")]
pub struct Loader<'info, T: ZeroCopy> {
    acc_info: AccountInfo<'info>,
    phantom: PhantomData<&'info T>,
}

#[allow(deprecated)]
impl<'info, T: ZeroCopy + fmt::Debug> fmt::Debug for Loader<'info, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Loader")
            .field("acc_info", &self.acc_info)
            .field("phantom", &self.phantom)
            .finish()
    }
}

#[allow(deprecated)]
impl<'info, T: ZeroCopy> Loader<'info, T> {
    fn new(acc_info: AccountInfo<'info>) -> Loader<'info, T> {
        Self {
            acc_info,
            phantom: PhantomData,
        }
    }

    /// Constructs a new `Loader` from a previously initialized account.
    #[inline(never)]
    #[allow(deprecated)]
    pub fn try_from(
        program_id: &Pubkey,
        acc_info: &AccountInfo<'info>,
    ) -> Result<Loader<'info, T>> {
        if acc_info.owner != program_id {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*acc_info.owner, *program_id)));
        }
        let data: &[u8] = &acc_info.try_borrow_data()?;
        if data.len() < T::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }
        // Discriminator must match.
        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &T::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(Loader::new(acc_info.clone()))
    }

    /// Constructs a new `Loader` from an uninitialized account.
    #[allow(deprecated)]
    #[inline(never)]
    pub fn try_from_unchecked(
        program_id: &Pubkey,
        acc_info: &AccountInfo<'info>,
    ) -> Result<Loader<'info, T>> {
        if acc_info.owner != program_id {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*acc_info.owner, *program_id)));
        }
        Ok(Loader::new(acc_info.clone()))
    }

    /// Returns a Ref to the account data structure for reading.
    #[allow(deprecated)]
    pub fn load(&self) -> Result<Ref<T>> {
        let data = self.acc_info.try_borrow_data()?;
        if data.len() < T::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }

        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &T::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(Ref::map(data, |data| bytemuck::from_bytes(&data[8..])))
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    #[allow(deprecated)]
    pub fn load_mut(&self) -> Result<RefMut<T>> {
        // AccountInfo api allows you to borrow mut even if the account isn't
        // writable, so add this check for a better dev experience.
        if !self.acc_info.is_writable {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let data = self.acc_info.try_borrow_mut_data()?;
        if data.len() < T::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }

        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &T::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(RefMut::map(data, |data| {
            bytemuck::from_bytes_mut(&mut data.deref_mut()[8..])
        }))
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    /// Should only be called once, when the account is being initialized.
    #[allow(deprecated)]
    pub fn load_init(&self) -> Result<RefMut<T>> {
        // AccountInfo api allows you to borrow mut even if the account isn't
        // writable, so add this check for a better dev experience.
        if !self.acc_info.is_writable {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let data = self.acc_info.try_borrow_mut_data()?;

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

#[allow(deprecated)]
impl<'info, T: ZeroCopy> Accounts<'info> for Loader<'info, T> {
    #[inline(never)]
    fn try_accounts(
        program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut BTreeMap<String, u8>,
        _reallocs: &mut BTreeSet<Pubkey>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        let l = Loader::try_from(program_id, account)?;
        Ok(l)
    }
}

#[allow(deprecated)]
impl<'info, T: ZeroCopy> AccountsExit<'info> for Loader<'info, T> {
    // The account *cannot* be loaded when this is called.
    fn exit(&self, _program_id: &Pubkey) -> Result<()> {
        let mut data = self.acc_info.try_borrow_mut_data()?;
        let dst: &mut [u8] = &mut data;
        let mut writer = BpfWriter::new(dst);
        writer.write_all(&T::discriminator()).unwrap();
        Ok(())
    }
}

/// This function is for INTERNAL USE ONLY.
/// Do NOT use this function in a program.
/// Manual closing of `Loader<'info, T>` types is NOT supported.
///
/// Details: Using `close` with `Loader<'info, T>` is not safe because
/// it requires the `mut` constraint but for that type the constraint
/// overwrites the "closed account" discriminator at the end of the instruction.
#[allow(deprecated)]
impl<'info, T: ZeroCopy> AccountsClose<'info> for Loader<'info, T> {
    fn close(&self, sol_destination: AccountInfo<'info>) -> Result<()> {
        crate::common::close(self.to_account_info(), sol_destination)
    }
}

#[allow(deprecated)]
impl<'info, T: ZeroCopy> ToAccountMetas for Loader<'info, T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.acc_info.is_signer);
        let meta = match self.acc_info.is_writable {
            false => AccountMeta::new_readonly(*self.acc_info.key, is_signer),
            true => AccountMeta::new(*self.acc_info.key, is_signer),
        };
        vec![meta]
    }
}

#[allow(deprecated)]
impl<'info, T: ZeroCopy> AsRef<AccountInfo<'info>> for Loader<'info, T> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.acc_info
    }
}

#[allow(deprecated)]
impl<'info, T: ZeroCopy> ToAccountInfos<'info> for Loader<'info, T> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.acc_info.clone()]
    }
}

#[allow(deprecated)]
impl<'info, T: ZeroCopy> Key for Loader<'info, T> {
    fn key(&self) -> Pubkey {
        *self.acc_info.key
    }
}
