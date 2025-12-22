//! Type facilitating on demand zero copy deserialization.

use crate::pinocchio_runtime::account_info::{AccountInfo, Ref, RefMut};

use crate::bpf_writer::BpfWriter;
use crate::error::{Error, ErrorCode};
use crate::pinocchio_runtime::instruction::AccountMeta;
use crate::pinocchio_runtime::pubkey::Pubkey;
use crate::{
    Accounts, AccountsClose, AccountsExit, Key, Owner, Result, ToAccountInfo, ToAccountInfos,
    ToAccountMetas, ZeroCopy,
};

use std::collections::BTreeSet;
use std::fmt;
use std::io::Write;
use std::marker::PhantomData;
use std::mem;

/// Type facilitating on demand zero copy deserialization.
///
/// Note that using accounts in this way is distinctly different from using,
/// for example, the [`Account`](crate::accounts::account::Account). Namely,
/// one must call
/// - `load_init` after initializing an account (this will ignore the missing
///   account discriminator that gets added only after the user's instruction code)
/// - `load` when the account is not mutable
/// - `load_mut` when the account is mutable
///
/// For more details on zero-copy-deserialization, see the
/// [`account`](crate::account) attribute.
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
pub struct AccountLoader<T: ZeroCopy + Owner> {
    acc_info: AccountInfo,
    phantom: PhantomData<T>,
}

impl<T: ZeroCopy + Owner + fmt::Debug> fmt::Debug for AccountLoader<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountLoader")
            .field("acc_info", &self.acc_info)
            .field("phantom", &self.phantom)
            .finish()
    }
}

impl<T: ZeroCopy + Owner> AccountLoader<T> {
    fn new(acc_info: AccountInfo) -> AccountLoader<T> {
        Self {
            acc_info,
            phantom: PhantomData,
        }
    }

    /// Constructs a new `Loader` from a previously initialized account.
    #[inline(never)]
    pub fn try_from(acc_info: AccountInfo) -> Result<AccountLoader<T>> {
        if acc_info.owned_by(&T::owner()) {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((unsafe { *acc_info.owner() }, T::owner())));
        }

        let data = &acc_info.try_borrow()?;
        let disc = T::DISCRIMINATOR;
        if data.len() < disc.len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }

        let given_disc = &data[..disc.len()];
        if given_disc != disc {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(AccountLoader::new(acc_info))
    }

    /// Constructs a new `Loader` from an uninitialized account.
    #[inline(never)]
    pub fn try_from_unchecked(
        _program_id: &Pubkey,
        acc_info: AccountInfo,
    ) -> Result<AccountLoader<T>> {
        if acc_info.owned_by(&T::owner()) {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((unsafe { *acc_info.owner() }, T::owner())));
        }
        Ok(AccountLoader::new(acc_info))
    }

    /// Returns a Ref to the account data structure for reading.
    pub fn load(&self) -> Result<Ref<'_, T>> {
        let data: Ref<'_, [u8]> = self.acc_info.try_borrow()?;
        let disc = T::DISCRIMINATOR;
        if data.len() < disc.len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }

        let given_disc = &data[..disc.len()];
        if given_disc != disc {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(Ref::map(data, |data: &[u8]| {
            bytemuck::from_bytes(&data[disc.len()..mem::size_of::<T>() + disc.len()])
        }))
    }
    /// Returns a `RefMut` to the account data structure for reading or writing.
    pub fn load_mut(&self) -> Result<RefMut<'_, T>> {
        // AccountInfo api allows you to borrow mut even if the account isn't
        // writable, so add this check for a better dev experience.
        if !self.acc_info.is_writable() {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let data: RefMut<'_, [u8]> = self.acc_info.try_borrow_mut()?;
        let disc = T::DISCRIMINATOR;
        if data.len() < disc.len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }

        let given_disc = &data[..disc.len()];
        if given_disc != disc {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Ok(RefMut::map(data, |data: &mut [u8]| {
            bytemuck::from_bytes_mut(&mut data[disc.len()..mem::size_of::<T>() + disc.len()])
        }))
    }

    /// Returns a `RefMut` to the account data structure for reading or writing.
    /// Should only be called once, when the account is being initialized.
    pub fn load_init(&self) -> Result<RefMut<'_, T>> {
        // AccountInfo api allows you to borrow mut even if the account isn't
        // writable, so add this check for a better dev experience.
        if !self.acc_info.is_writable() {
            return Err(ErrorCode::AccountNotMutable.into());
        }

        let data: RefMut<'_, [u8]> = self.acc_info.try_borrow_mut()?;

        // The discriminator should be zero, since we're initializing.
        let disc = T::DISCRIMINATOR;
        let given_disc = &data[..disc.len()];
        let has_disc = given_disc.iter().any(|b| *b != 0);
        if has_disc {
            return Err(ErrorCode::AccountDiscriminatorAlreadySet.into());
        }

        Ok(RefMut::map(data, |data: &mut [u8]| {
            bytemuck::from_bytes_mut(&mut data[disc.len()..mem::size_of::<T>() + disc.len()])
        }))
    }
}

impl<'info, B, T: ZeroCopy + Owner> Accounts<'info, B> for AccountLoader<T> {
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &[AccountInfo],
        _ix_data: &[u8],
        _bumps: &mut B,
        _reallocs: &mut BTreeSet<Pubkey>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = accounts[0];
        *accounts = &accounts[1..];
        let l = AccountLoader::try_from(account)?;
        Ok(l)
    }
}

impl<'info, T: ZeroCopy + Owner> AccountsExit<'info> for AccountLoader<T> {
    // The account *cannot* be loaded when this is called.
    fn exit(&self, program_id: &Pubkey) -> Result<()> {
        // Only persist if the owner is the current program and the account is not closed.
        if &T::owner() == program_id && !crate::common::is_closed(&self.acc_info) {
            let mut data = self.acc_info.try_borrow_mut()?;
            let dst: &mut [u8] = &mut data;
            let mut writer = BpfWriter::new(dst);
            writer.write_all(T::DISCRIMINATOR).unwrap();
        }
        Ok(())
    }
}

impl<T: ZeroCopy + Owner> AccountsClose for AccountLoader<T> {
    fn close(&self, sol_destination: AccountInfo) -> Result<()> {
        crate::common::close(self.to_account_info(), sol_destination)
    }
}

impl<'info, T: ZeroCopy + Owner> ToAccountMetas<'info> for AccountLoader<T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta<'_>> {
        let is_signer = is_signer.unwrap_or(self.acc_info.is_signer());
        let meta = match (self.acc_info.is_writable(), is_signer) {
            (false, false) => AccountMeta::readonly(self.acc_info.address()),
            (false, true) => AccountMeta::readonly_signer(self.acc_info.address()),
            (true, false) => AccountMeta::writable(self.acc_info.address()),
            (true, true) => AccountMeta::writable_signer(self.acc_info.address()),
        };
        vec![meta]
    }
}

impl<T: ZeroCopy + Owner> AsRef<AccountInfo> for AccountLoader<T> {
    fn as_ref(&self) -> &AccountInfo {
        &self.acc_info
    }
}

impl<T: ZeroCopy + Owner> ToAccountInfos for AccountLoader<T> {
    fn to_account_infos(&self) -> Vec<AccountInfo> {
        vec![self.acc_info]
    }
}

impl<T: ZeroCopy + Owner> Key for AccountLoader<T> {
    fn key(&self) -> Pubkey {
        self.acc_info.key()
    }
}
