//! Type facilitating on demand zero copy deserialization.

use crate::bpf_writer::BpfWriter;
use crate::error::{Error, ErrorCode};
use crate::{
    Accounts, AccountsClose, AccountsExit, Key, Owner, Result, ToAccountInfo, ToAccountInfos,
    ToAccountMetas, ZeroCopy,
};
use arrayref::array_ref;
use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use std::cell::{Ref, RefMut};
use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use std::marker::PhantomData;
use std::mem;
use std::ops::DerefMut;

/// Type facilitating on demand zero copy deserialization.
///
/// Note that using accounts in this way is distinctly different from using,
/// for example, the [`Account`](./struct.Account.html). Namely,
/// one must call
/// - `load_init` after initializing an account (this will ignore the missing
/// account discriminator that gets added only after the user's instruction code)
/// - `load` when the account is not mutable
/// - `load_mut` when the account is mutable
///
/// For more details on zero-copy-deserialization, see the
/// [`account`](./attr.account.html) attribute.
/// <p style=";padding:0.75em;border: 1px solid #ee6868">
/// <strong>⚠️ </strong> When using this type it's important to be mindful
/// of any calls to the <code>load</code> functions so as not to
/// induce a <code>RefCell</code> panic, especially when sharing accounts across CPI
/// boundaries. When in doubt, one should make sure all refs resulting from
/// a call to a <code>load</code> function are dropped before CPI.
/// This can be done explicitly by calling <code>drop(my_var)</code> or implicitly
/// by wrapping the code using the <code>Ref</code> in braces <code>{..}</code> or
/// moving it into its own function.
/// </p>
///
/// # Example
/// ```ignore
/// use anchor_lang::prelude::*;
///
/// declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
///
/// #[program]
/// pub mod bar {
///     use super::*;
///
///     pub fn create_bar(ctx: Context<CreateBar>, data: u64) -> Result<()> {
///         let bar = &mut ctx.accounts.bar.load_init()?;
///         bar.authority = ctx.accounts.authority.key();
///         bar.data = data;
///         Ok(())
///     }
///
///     pub fn update_bar(ctx: Context<UpdateBar>, data: u64) -> Result<()> {
///         (*ctx.accounts.bar.load_mut()?).data = data;
///         Ok(())
///     }
/// }
///
/// #[account(zero_copy)]
/// #[derive(Default)]
/// pub struct Bar {
///     authority: Pubkey,
///     data: u64
/// }
///
/// #[derive(Accounts)]
/// pub struct CreateBar<'info> {
///     #[account(
///         init,
///         payer = authority
///     )]
///     bar: AccountLoader<'info, Bar>,
///     #[account(mut)]
///     authority: Signer<'info>,
///     system_program: AccountInfo<'info>,
/// }
///
/// #[derive(Accounts)]
/// pub struct UpdateBar<'info> {
///     #[account(
///         mut,
///         has_one = authority,
///     )]
///     pub bar: AccountLoader<'info, Bar>,
///     pub authority: Signer<'info>,
/// }
/// ```
#[derive(Clone)]
pub struct AccountLoader<'info, T: ZeroCopy + Owner> {
    acc_info: AccountInfo<'info>,
    phantom: PhantomData<&'info T>,
}

impl<'info, T: ZeroCopy + Owner + fmt::Debug> fmt::Debug for AccountLoader<'info, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountLoader")
            .field("acc_info", &self.acc_info)
            .field("phantom", &self.phantom)
            .finish()
    }
}

impl<'info, T: ZeroCopy + Owner> AccountLoader<'info, T> {
    fn new(acc_info: AccountInfo<'info>) -> AccountLoader<'info, T> {
        Self {
            acc_info,
            phantom: PhantomData,
        }
    }

    /// Constructs a new `Loader` from a previously initialized account.
    #[inline(never)]
    pub fn try_from(acc_info: &AccountInfo<'info>) -> Result<AccountLoader<'info, T>> {
        if acc_info.owner != &T::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*acc_info.owner, T::owner())));
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

        Ok(AccountLoader::new(acc_info.clone()))
    }

    /// Constructs a new `Loader` from an uninitialized account.
    #[inline(never)]
    pub fn try_from_unchecked(
        _program_id: &Pubkey,
        acc_info: &AccountInfo<'info>,
    ) -> Result<AccountLoader<'info, T>> {
        if acc_info.owner != &T::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*acc_info.owner, T::owner())));
        }
        Ok(AccountLoader::new(acc_info.clone()))
    }

    /// Returns a Ref to the account data structure for reading.
    pub fn load(&self) -> Result<Ref<T>> {
        let data = self.acc_info.try_borrow_data()?;
        if data.len() < T::discriminator().len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }

        let disc_bytes = array_ref![data, 0, 8];
        if disc_bytes != &T::discriminator() {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(Ref::map(data, |data| {
            bytemuck::from_bytes(&data[8..mem::size_of::<T>() + 8])
        }))
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
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
            bytemuck::from_bytes_mut(&mut data.deref_mut()[8..mem::size_of::<T>() + 8])
        }))
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    /// Should only be called once, when the account is being initialized.
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
            bytemuck::from_bytes_mut(&mut data.deref_mut()[8..mem::size_of::<T>() + 8])
        }))
    }
}

impl<'info, T: ZeroCopy + Owner> Accounts<'info> for AccountLoader<'info, T> {
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut BTreeMap<String, u8>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        let l = AccountLoader::try_from(account)?;
        Ok(l)
    }
}

impl<'info, T: ZeroCopy + Owner> AccountsExit<'info> for AccountLoader<'info, T> {
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
/// Manual closing of `AccountLoader<'info, T>` types is NOT supported.
///
/// Details: Using `close` with `AccountLoader<'info, T>` is not safe because
/// it requires the `mut` constraint but for that type the constraint
/// overwrites the "closed account" discriminator at the end of the instruction.
impl<'info, T: ZeroCopy + Owner> AccountsClose<'info> for AccountLoader<'info, T> {
    fn close(&self, sol_destination: AccountInfo<'info>) -> Result<()> {
        crate::common::close(self.to_account_info(), sol_destination)
    }
}

impl<'info, T: ZeroCopy + Owner> ToAccountMetas for AccountLoader<'info, T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.acc_info.is_signer);
        let meta = match self.acc_info.is_writable {
            false => AccountMeta::new_readonly(*self.acc_info.key, is_signer),
            true => AccountMeta::new(*self.acc_info.key, is_signer),
        };
        vec![meta]
    }
}

impl<'info, T: ZeroCopy + Owner> AsRef<AccountInfo<'info>> for AccountLoader<'info, T> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.acc_info
    }
}

impl<'info, T: ZeroCopy + Owner> ToAccountInfos<'info> for AccountLoader<'info, T> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.acc_info.clone()]
    }
}

impl<'info, T: ZeroCopy + Owner> Key for AccountLoader<'info, T> {
    fn key(&self) -> Pubkey {
        *self.acc_info.key
    }
}
