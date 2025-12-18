// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use crate::prelude::*;
use crate::solana_program::pubkey::Pubkey;

pub use crate::solana_program::system_program::ID;

#[derive(Debug, Clone)]
pub struct System;

impl anchor_lang::Id for System {
    fn id() -> Pubkey {
        ID
    }
}

pub fn advance_nonce_account<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, AdvanceNonceAccount<'info>>,
) -> Result<()> {
    let ix = crate::solana_program::system_instruction::advance_nonce_account(
        ctx.accounts.nonce.key,
        ctx.accounts.authorized.key,
    );
    crate::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.nonce,
            ctx.accounts.recent_blockhashes,
            ctx.accounts.authorized,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct AdvanceNonceAccount<'info> {
    pub nonce: AccountInfo<'info>,
    pub authorized: AccountInfo<'info>,
    pub recent_blockhashes: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for AdvanceNonceAccount<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.nonce.to_owned(),
            self.recent_blockhashes.to_owned(),
            self.authorized.to_owned(),
        ]
    }
}

impl<'info> ToAccountMetas for AdvanceNonceAccount<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.nonce.key(), false),
            AccountMeta::new_readonly(self.recent_blockhashes.key(), false),
            AccountMeta::new_readonly(self.authorized.key(), true),
        ]
    }
}

pub fn allocate<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, Allocate<'info>>,
    space: u64,
) -> Result<()> {
    let ix = crate::solana_program::system_instruction::allocate(
        ctx.accounts.account_to_allocate.key,
        space,
    );
    crate::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.account_to_allocate],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct Allocate<'info> {
    pub account_to_allocate: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for Allocate<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.account_to_allocate.to_owned()]
    }
}

impl<'info> ToAccountMetas for Allocate<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![AccountMeta::new(self.account_to_allocate.key(), true)]
    }
}

pub fn allocate_with_seed<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, AllocateWithSeed<'info>>,
    seed: &str,
    space: u64,
    owner: &Pubkey,
) -> Result<()> {
    let ix = crate::solana_program::system_instruction::allocate_with_seed(
        ctx.accounts.account_to_allocate.key,
        ctx.accounts.base.key,
        seed,
        space,
        owner,
    );
    crate::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.account_to_allocate, ctx.accounts.base],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct AllocateWithSeed<'info> {
    pub account_to_allocate: AccountInfo<'info>,
    pub base: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for AllocateWithSeed<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.account_to_allocate.to_owned(), self.base.to_owned()]
    }
}

impl<'info> ToAccountMetas for AllocateWithSeed<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.account_to_allocate.key(), false),
            AccountMeta::new_readonly(self.base.key(), true),
        ]
    }
}

pub fn assign<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, Assign<'info>>,
    owner: &Pubkey,
) -> Result<()> {
    let ix = crate::solana_program::system_instruction::assign(
        ctx.accounts.account_to_assign.key,
        owner,
    );
    crate::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.account_to_assign],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct Assign<'info> {
    pub account_to_assign: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for Assign<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.account_to_assign.to_owned()]
    }
}

impl<'info> ToAccountMetas for Assign<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![AccountMeta::new(self.account_to_assign.key(), true)]
    }
}

pub fn assign_with_seed<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, AssignWithSeed<'info>>,
    seed: &str,
    owner: &Pubkey,
) -> Result<()> {
    let ix = crate::solana_program::system_instruction::assign_with_seed(
        ctx.accounts.account_to_assign.key,
        ctx.accounts.base.key,
        seed,
        owner,
    );
    crate::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.account_to_assign, ctx.accounts.base],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct AssignWithSeed<'info> {
    pub account_to_assign: AccountInfo<'info>,
    pub base: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for AssignWithSeed<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.account_to_assign.to_owned(), self.base.to_owned()]
    }
}

impl<'info> ToAccountMetas for AssignWithSeed<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.account_to_assign.key(), false),
            AccountMeta::new_readonly(self.base.key(), true),
        ]
    }
}

pub fn authorize_nonce_account<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, AuthorizeNonceAccount<'info>>,
    new_authority: &Pubkey,
) -> Result<()> {
    let ix = crate::solana_program::system_instruction::authorize_nonce_account(
        ctx.accounts.nonce.key,
        ctx.accounts.authorized.key,
        new_authority,
    );
    crate::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.nonce, ctx.accounts.authorized],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct AuthorizeNonceAccount<'info> {
    pub nonce: AccountInfo<'info>,
    pub authorized: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for AuthorizeNonceAccount<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.nonce.to_owned(), self.authorized.to_owned()]
    }
}

impl<'info> ToAccountMetas for AuthorizeNonceAccount<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.nonce.key(), false),
            AccountMeta::new_readonly(self.authorized.key(), true),
        ]
    }
}

pub fn create_account<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CreateAccount<'info>>,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Result<()> {
    let ix = crate::solana_program::system_instruction::create_account(
        ctx.accounts.from.key,
        ctx.accounts.to.key,
        lamports,
        space,
        owner,
    );
    crate::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.from, ctx.accounts.to],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct CreateAccount<'info> {
    pub from: AccountInfo<'info>,
    pub to: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for CreateAccount<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.from.to_owned(), self.to.to_owned()]
    }
}

impl<'info> ToAccountMetas for CreateAccount<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.from.key(), true),
            AccountMeta::new(self.to.key(), true),
        ]
    }
}

pub fn create_account_with_seed<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CreateAccountWithSeed<'info>>,
    seed: &str,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Result<()> {
    let ix = crate::solana_program::system_instruction::create_account_with_seed(
        ctx.accounts.from.key,
        ctx.accounts.to.key,
        ctx.accounts.base.key,
        seed,
        lamports,
        space,
        owner,
    );
    crate::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.from, ctx.accounts.to, ctx.accounts.base],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct CreateAccountWithSeed<'info> {
    pub from: AccountInfo<'info>,
    pub to: AccountInfo<'info>,
    pub base: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for CreateAccountWithSeed<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.from.to_owned(),
            self.to.to_owned(),
            self.base.to_owned(),
        ]
    }
}

impl<'info> ToAccountMetas for CreateAccountWithSeed<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.from.key(), true),
            AccountMeta::new(self.to.key(), false),
            AccountMeta::new_readonly(self.to.key(), true),
        ]
    }
}

pub fn create_nonce_account<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CreateNonceAccount<'info>>,
    lamports: u64,
    authority: &Pubkey,
) -> Result<()> {
    let ixs = crate::solana_program::system_instruction::create_nonce_account(
        ctx.accounts.from.key,
        ctx.accounts.nonce.key,
        authority,
        lamports,
    );
    crate::solana_program::program::invoke_signed(
        &ixs[0],
        &[ctx.accounts.from, ctx.accounts.nonce.clone()],
        ctx.signer_seeds,
    )?;

    crate::solana_program::program::invoke_signed(
        &ixs[1],
        &[
            ctx.accounts.nonce,
            ctx.accounts.recent_blockhashes,
            ctx.accounts.rent,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct CreateNonceAccount<'info> {
    pub from: AccountInfo<'info>,
    pub nonce: AccountInfo<'info>,
    pub recent_blockhashes: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for CreateNonceAccount<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.from.to_owned(),
            self.nonce.to_owned(),
            self.recent_blockhashes.to_owned(),
            self.rent.to_owned(),
        ]
    }
}

impl<'info> ToAccountMetas for CreateNonceAccount<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.from.key(), true),
            AccountMeta::new(self.nonce.key(), false),
            AccountMeta::new_readonly(self.recent_blockhashes.key(), false),
            AccountMeta::new_readonly(self.rent.key(), false),
        ]
    }
}

pub fn create_nonce_account_with_seed<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, CreateNonceAccountWithSeed<'info>>,
    lamports: u64,
    seed: &str,
    authority: &Pubkey,
) -> Result<()> {
    let ixs = crate::solana_program::system_instruction::create_nonce_account_with_seed(
        ctx.accounts.from.key,
        ctx.accounts.nonce.key,
        ctx.accounts.base.key,
        seed,
        authority,
        lamports,
    );
    crate::solana_program::program::invoke_signed(
        &ixs[0],
        &[
            ctx.accounts.from,
            ctx.accounts.nonce.clone(),
            ctx.accounts.base,
        ],
        ctx.signer_seeds,
    )?;

    crate::solana_program::program::invoke_signed(
        &ixs[1],
        &[
            ctx.accounts.nonce,
            ctx.accounts.recent_blockhashes,
            ctx.accounts.rent,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct CreateNonceAccountWithSeed<'info> {
    pub from: AccountInfo<'info>,
    pub nonce: AccountInfo<'info>,
    pub base: AccountInfo<'info>,
    pub recent_blockhashes: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for CreateNonceAccountWithSeed<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.from.to_owned(),
            self.nonce.to_owned(),
            self.base.to_owned(),
            self.recent_blockhashes.to_owned(),
            self.rent.to_owned(),
        ]
    }
}

impl<'info> ToAccountMetas for CreateNonceAccountWithSeed<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.from.key(), false),
            AccountMeta::new(self.nonce.key(), false),
            AccountMeta::new_readonly(self.base.key(), true),
            AccountMeta::new_readonly(self.recent_blockhashes.key(), false),
            AccountMeta::new_readonly(self.rent.key(), false),
        ]
    }
}

pub fn transfer<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, Transfer<'info>>,
    lamports: u64,
) -> Result<()> {
    let ix = crate::solana_program::system_instruction::transfer(
        ctx.accounts.from.key,
        ctx.accounts.to.key,
        lamports,
    );
    crate::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.from, ctx.accounts.to],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct Transfer<'info> {
    pub from: AccountInfo<'info>,
    pub to: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for Transfer<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.from.to_owned(), self.to.to_owned()]
    }
}

impl<'info> ToAccountMetas for Transfer<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.from.key(), true),
            AccountMeta::new(self.to.key(), false),
        ]
    }
}

pub fn transfer_with_seed<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, TransferWithSeed<'info>>,
    from_seed: String,
    from_owner: &Pubkey,
    lamports: u64,
) -> Result<()> {
    let ix = crate::solana_program::system_instruction::transfer_with_seed(
        ctx.accounts.from.key,
        ctx.accounts.base.key,
        from_seed,
        from_owner,
        ctx.accounts.to.key,
        lamports,
    );
    crate::solana_program::program::invoke_signed(
        &ix,
        &[ctx.accounts.from, ctx.accounts.base, ctx.accounts.to],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct TransferWithSeed<'info> {
    pub from: AccountInfo<'info>,
    pub base: AccountInfo<'info>,
    pub to: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for TransferWithSeed<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.from.to_owned(),
            self.base.to_owned(),
            self.to.to_owned(),
        ]
    }
}

impl<'info> ToAccountMetas for TransferWithSeed<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.from.key(), false),
            AccountMeta::new_readonly(self.base.key(), true),
            AccountMeta::new(self.to.key(), false),
        ]
    }
}

pub fn withdraw_nonce_account<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, WithdrawNonceAccount<'info>>,
    lamports: u64,
) -> Result<()> {
    let ix = crate::solana_program::system_instruction::withdraw_nonce_account(
        ctx.accounts.nonce.key,
        ctx.accounts.authorized.key,
        ctx.accounts.to.key,
        lamports,
    );
    crate::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.nonce,
            ctx.accounts.to,
            ctx.accounts.recent_blockhashes,
            ctx.accounts.rent,
            ctx.accounts.authorized,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub struct WithdrawNonceAccount<'info> {
    pub nonce: AccountInfo<'info>,
    pub to: AccountInfo<'info>,
    pub recent_blockhashes: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
    pub authorized: AccountInfo<'info>,
}

impl<'info> ToAccountInfos<'info> for WithdrawNonceAccount<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.nonce.to_owned(),
            self.to.to_owned(),
            self.recent_blockhashes.to_owned(),
            self.rent.to_owned(),
            self.authorized.to_owned(),
        ]
    }
}

impl<'info> ToAccountMetas for WithdrawNonceAccount<'info> {
    fn to_account_metas(&self, _: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.nonce.key(), false),
            AccountMeta::new(self.to.key(), false),
            AccountMeta::new_readonly(self.recent_blockhashes.key(), false),
            AccountMeta::new_readonly(self.rent.key(), false),
            AccountMeta::new_readonly(self.authorized.key(), true),
        ]
    }
}
