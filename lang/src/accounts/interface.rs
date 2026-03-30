//! Type validating that the account is one of a set of given Programs

use {
    crate::{
        accounts::account::{Account, AnyProgram, Program},
        error::{Error, ErrorCode},
        pinocchio_runtime::{account_view::AccountView, instruction::AccountMeta, pubkey::Pubkey},
        AccountDeserialize, Accounts, AccountsExit, CheckId, Key, Result, ToAccountMetas,
        ToAccountViews,
    },
    std::{collections::BTreeSet, marker::PhantomData, ops::Deref},
};

/// Type validating that the account is one of a set of given Programs
///
/// The `Interface` wraps over [`Program`], allowing for
/// multiple possible program ids. Useful for any program that implements an
/// instruction interface. For example, spl-token and spl-token-2022 both implement
/// the spl-token interface.
///
/// # Table of Contents
/// - [Basic Functionality](#basic-functionality)
/// - [Out of the Box Types](#out-of-the-box-types)
///
/// # Basic Functionality
///
/// Checks:
///
/// - `expected_programs.contains(account_info.key)`
/// - `account_info.executable == true`
///
/// # Example
/// ```ignore
/// #[program]
/// mod my_program {
///     fn set_admin_settings(...){...}
/// }
///
/// #[account]
/// #[derive(Default)]
/// pub struct AdminSettings {
///     ...
/// }
///
/// #[derive(Accounts)]
/// pub struct SetAdminSettings<'info> {
///     #[account(mut, seeds = [b"admin"], bump)]
///     pub admin_settings: Account<'info, AdminSettings>,
///     #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
///     pub program: Interface<'info, MyProgram>,
///     #[account(constraint = program_data.upgrade_authority_address == Some(authority.key()))]
///     pub program_data: Account<'info, ProgramData>,
///     pub authority: Signer<'info>,
/// }
/// ```
/// The given program has a function with which the upgrade authority can set admin settings.
///
/// The required constraints are as follows:
///
/// - `program` is the account of the program itself.
///   Its constraint checks that `program_data` is the account that contains the program's upgrade authority.
///   Implicitly, this checks that `program` is a BPFUpgradeable program (`program.programdata_address()?`
///   will be `None` if it's not).
/// - `program_data`'s constraint checks that its upgrade authority is the `authority` account.
/// - Finally, `authority` needs to sign the transaction.
///
/// # Out of the Box Types
///
/// Between the [`anchor_lang`](https://docs.rs/anchor-lang/latest/anchor_lang) and [`anchor_spl`](https://docs.rs/anchor_spl/latest/anchor_spl) crates,
/// the following `Interface` types are provided out of the box:
///
/// - [`TokenInterface`](https://docs.rs/anchor-spl/latest/anchor_spl/token_interface/struct.TokenInterface.html)
///
#[derive(Clone)]
pub struct Interface<T> {
    program: Account<Program<AnyProgram>>,
    _marker: PhantomData<T>,
}
impl<T> Interface<T> {
    pub(crate) fn new(program: Account<Program<AnyProgram>>) -> Self {
        Self {
            program,
            _marker: PhantomData,
        }
    }
    pub fn programdata_address(&self) -> Result<Option<Pubkey>> {
        self.program.programdata_address()
    }
}
impl<T: CheckId> TryFrom<&AccountView> for Interface<T> {
    type Error = Error;
    /// Deserializes the given `info` into a `Program`.
    fn try_from(info: &AccountView) -> Result<Self> {
        T::check_id(info.address())?;
        Ok(Self::new(Account::try_from(*info)?))
    }
}
impl<T> Deref for Interface<T> {
    type Target = AccountView;
    fn deref(&self) -> &Self::Target {
        self.program.as_ref()
    }
}
impl<T> AsRef<AccountView> for Interface<T> {
    fn as_ref(&self) -> &AccountView {
        self.program.as_ref()
    }
}

impl<'info, B, T: CheckId> Accounts<'info, B> for Interface<T> {
    #[inline(never)]
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &'info [AccountView],
        _ix_data: &[u8],
        _bumps: &mut B,
        _reallocs: &mut BTreeSet<Pubkey>,
    ) -> Result<Self> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
        Self::try_from(account)
    }
}

impl<T> ToAccountMetas for Interface<T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta<'_>> {
        self.program.to_account_metas(is_signer)
    }
}

impl<T> ToAccountViews for Interface<T> {
    fn to_account_views(&self) -> Vec<AccountView> {
        self.program.to_account_views()
    }
}

impl<'info, T: AccountDeserialize> AccountsExit<'info> for Interface<T> {}

impl<T: AccountDeserialize> Key for Interface<T> {
    fn key(&self) -> Pubkey {
        self.program.key()
    }
}
