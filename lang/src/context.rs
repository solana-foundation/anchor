use crate::{Accounts, ToAccountInfos, ToAccountMetas};
use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use std::fmt;

/// Provides non-argument inputs to the program.
pub struct Context<'a, 'b, 'c, 'info, T> {
    /// Currently executing program id.
    pub program_id: &'a Pubkey,
    /// Deserialized accounts.
    pub accounts: &'b mut T,
    /// Remaining accounts given but not deserialized or validated.
    /// Be very careful when using this directly.
    pub remaining_accounts: &'c [AccountInfo<'info>],
}

impl<'a, 'b, 'c, 'info, T: fmt::Debug> fmt::Debug for Context<'a, 'b, 'c, 'info, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("program_id", &self.program_id)
            .field("accounts", &self.accounts)
            .field("remaining_accounts", &self.remaining_accounts)
            .finish()
    }
}

impl<'a, 'b, 'c, 'info, T: Accounts<'info>> Context<'a, 'b, 'c, 'info, T> {
    pub fn new(
        program_id: &'a Pubkey,
        accounts: &'b mut T,
        remaining_accounts: &'c [AccountInfo<'info>],
    ) -> Self {
        Self {
            program_id,
            accounts,
            remaining_accounts,
        }
    }
}

/// Context specifying non-argument inputs for cross-program-invocations.
pub struct CpiContext<'a, 'b, 'c, 'info, T>
where
    T: ToAccountMetas + ToAccountInfos<'info>,
{
    pub accounts: T,
    pub remaining_accounts: Vec<AccountInfo<'info>>,
    pub program: AccountInfo<'info>,
    pub signer_seeds: &'a [&'b [&'c [u8]]],
}

impl<'a, 'b, 'c, 'info, T> CpiContext<'a, 'b, 'c, 'info, T>
where
    T: ToAccountMetas + ToAccountInfos<'info>,
{
    pub fn new(program: AccountInfo<'info>, accounts: T) -> Self {
        Self {
            accounts,
            program,
            remaining_accounts: Vec::new(),
            signer_seeds: &[],
        }
    }

    pub fn new_with_signer(
        program: AccountInfo<'info>,
        accounts: T,
        signer_seeds: &'a [&'b [&'c [u8]]],
    ) -> Self {
        Self {
            accounts,
            program,
            signer_seeds,
            remaining_accounts: Vec::new(),
        }
    }

    pub fn with_signer(mut self, signer_seeds: &'a [&'b [&'c [u8]]]) -> Self {
        self.signer_seeds = signer_seeds;
        self
    }

    pub fn with_remaining_accounts(mut self, ra: Vec<AccountInfo<'info>>) -> Self {
        self.remaining_accounts = ra;
        self
    }
}

impl<'info, T: ToAccountInfos<'info> + ToAccountMetas> ToAccountInfos<'info>
    for CpiContext<'_, '_, '_, 'info, T>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        let mut infos = self.accounts.to_account_infos();
        infos.extend_from_slice(&self.remaining_accounts);
        infos.push(self.program.clone());
        infos
    }
}

impl<'info, T: ToAccountInfos<'info> + ToAccountMetas> ToAccountMetas
    for CpiContext<'_, '_, '_, 'info, T>
{
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let mut metas = self.accounts.to_account_metas(is_signer);
        metas.append(
            &mut self
                .remaining_accounts
                .iter()
                .map(|acc| match acc.is_writable {
                    false => AccountMeta::new_readonly(*acc.key, acc.is_signer),
                    true => AccountMeta::new(*acc.key, acc.is_signer),
                })
                .collect(),
        );
        metas
    }
}

/// Context specifying non-argument inputs for cross-program-invocations
/// targeted at program state instructions.
#[deprecated]
pub struct CpiStateContext<'a, 'b, 'c, 'info, T: Accounts<'info>> {
    state: AccountInfo<'info>,
    cpi_ctx: CpiContext<'a, 'b, 'c, 'info, T>,
}

#[allow(deprecated)]
impl<'a, 'b, 'c, 'info, T: Accounts<'info>> CpiStateContext<'a, 'b, 'c, 'info, T> {
    pub fn new(program: AccountInfo<'info>, state: AccountInfo<'info>, accounts: T) -> Self {
        Self {
            state,
            cpi_ctx: CpiContext {
                accounts,
                program,
                signer_seeds: &[],
                remaining_accounts: Vec::new(),
            },
        }
    }

    pub fn new_with_signer(
        program: AccountInfo<'info>,
        state: AccountInfo<'info>,
        accounts: T,
        signer_seeds: &'a [&'b [&'c [u8]]],
    ) -> Self {
        Self {
            state,
            cpi_ctx: CpiContext {
                accounts,
                program,
                signer_seeds,
                remaining_accounts: Vec::new(),
            },
        }
    }

    pub fn with_signer(mut self, signer_seeds: &'a [&'b [&'c [u8]]]) -> Self {
        self.cpi_ctx = self.cpi_ctx.with_signer(signer_seeds);
        self
    }

    pub fn program(&self) -> &AccountInfo<'info> {
        &self.cpi_ctx.program
    }

    pub fn signer_seeds(&self) -> &[&[&[u8]]] {
        self.cpi_ctx.signer_seeds
    }
}

#[allow(deprecated)]
impl<'a, 'b, 'c, 'info, T: Accounts<'info>> ToAccountMetas
    for CpiStateContext<'a, 'b, 'c, 'info, T>
{
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        // State account is always first for state instructions.
        let mut metas = vec![match self.state.is_writable {
            false => AccountMeta::new_readonly(*self.state.key, false),
            true => AccountMeta::new(*self.state.key, false),
        }];
        metas.append(&mut self.cpi_ctx.accounts.to_account_metas(is_signer));
        metas
    }
}

#[allow(deprecated)]
impl<'a, 'b, 'c, 'info, T: Accounts<'info>> ToAccountInfos<'info>
    for CpiStateContext<'a, 'b, 'c, 'info, T>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        let mut infos = self.cpi_ctx.accounts.to_account_infos();
        infos.push(self.state.clone());
        infos.push(self.cpi_ctx.program.clone());
        infos
    }
}
