use {
    crate::{
        error::{Error, ErrorCode},
        solana_program::{
            account_info::AccountInfo, instruction::AccountMeta, pubkey::Pubkey, system_program,
        },
        Accounts, AccountsExit, CustomCodec, Owner, Result, ToAccountInfos, ToAccountMetas,
    },
    std::{
        collections::BTreeSet,
        ops::{Deref, DerefMut},
    },
};

/// Wrapper around [`AccountInfo`] that verifies program ownership and
/// deserializes account data using a [`CustomCodec`] instead of the default
/// Borsh-based path.
///
/// Use this in place of [`Account`](crate::accounts::account::Account) when
/// your account type implements [`CustomCodec`] rather than
/// `AccountSerialize + AccountDeserialize`.
#[derive(Clone)]
pub struct CustomAccount<'info, T: CustomCodec + Clone> {
    account: T,
    info: &'info AccountInfo<'info>,
}

impl<'a, T: CustomCodec + Clone> CustomAccount<'a, T> {
    pub(crate) fn new(info: &'a AccountInfo<'a>, account: T) -> Self {
        Self { info, account }
    }

    pub(crate) fn exit_with_expected_owner(
        &self,
        expected_owner: &Pubkey,
        program_id: &Pubkey,
    ) -> Result<()> {
        // Only persist if the owner is the current program and the account is not closed.
        if expected_owner == program_id && !crate::common::is_closed(self.info) {
            let mut data = self.info.try_borrow_mut_data()?;
            let disc = T::DISCRIMINATOR;
            data[..disc.len()].copy_from_slice(disc);
            self.account.encode(&mut data[disc.len()..])?;
        }
        Ok(())
    }

    pub fn into_inner(self) -> T {
        self.account
    }

    pub fn set_inner(&mut self, inner: T) {
        self.account = inner;
    }
}

impl<'a, T: CustomCodec + Owner + Clone> CustomAccount<'a, T> {
    /// Deserializes the given `info` into a `CustomAccount`.
    #[inline(never)]
    pub fn try_from(info: &'a AccountInfo<'a>) -> Result<CustomAccount<'a, T>> {
        if info.owner == &system_program::ID && info.lamports() == 0 {
            return Err(ErrorCode::AccountNotInitialized.into());
        }
        if info.owner != &T::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*info.owner, T::owner())));
        }

        let data: &[u8] = &info.try_borrow_data()?;
        let disc = T::DISCRIMINATOR;

        if data.len() < disc.len() {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }
        if &data[..disc.len()] != disc {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }
        Ok(CustomAccount::new(info, T::decode(&data[disc.len()..])?))
    }
}

impl<'info, B, T: CustomCodec + Owner + Clone> Accounts<'info, B> for CustomAccount<'info, T>
where
    T: CustomCodec + Owner + Clone,
{
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &'info [AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut B,
        _reallocs: &mut BTreeSet<Pubkey>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        CustomAccount::try_from(account)
    }
}

impl<T: CustomCodec + Clone> Deref for CustomAccount<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &(self).account
    }
}

impl<T: CustomCodec + Clone> DerefMut for CustomAccount<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        #[cfg(feature = "anchor-debug")]
        if !self.info.is_writable {
            crate::solana_program::msg!("The given Account is not mutable");
            panic!();
        }
        &mut self.account
    }
}

impl<'info, T: CustomCodec + Owner + Clone> AccountsExit<'info> for CustomAccount<'info, T> {
    fn exit(&self, program_id: &Pubkey) -> Result<()> {
        self.exit_with_expected_owner(&T::owner(), program_id)
    }
}

impl<'info, T: CustomCodec + Clone> ToAccountInfos<'info> for CustomAccount<'info, T> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<T: CustomCodec + Clone> ToAccountMetas for CustomAccount<'_, T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.info.is_signer);
        let meta = match self.info.is_writable {
            false => AccountMeta::new_readonly(*self.info.key, is_signer),
            true => AccountMeta::new(*self.info.key, is_signer),
        };
        vec![meta]
    }
}
