use anchor_lang::__private::bytemuck::Pod;
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use std::ops::Deref;

use spl_token_2022_interface::extension::{Extension, ExtensionType};

pub use crate::token_2022::*;
#[cfg(feature = "token_2022_extensions")]
pub use crate::token_2022_extensions::*;

static IDS: [Pubkey; 2] = [pinocchio_token::ID, pinocchio_token_2022::ID];

#[derive()]
pub struct TokenAccount(pinocchio_token_2022::state::TokenAccount);

impl anchor_lang::AccountDeserialize for TokenAccount {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let token_account_ref =
            unsafe { pinocchio_token_2022::state::TokenAccount::from_bytes_unchecked(buf) };
        let token_account = unsafe { std::ptr::read(token_account_ref) };
        Ok(TokenAccount(token_account))
    }
}

impl anchor_lang::AccountSerialize for TokenAccount {}

impl anchor_lang::Owners for TokenAccount {
    fn owners() -> &'static [Pubkey] {
        &IDS
    }
}

impl Deref for TokenAccount {
    type Target = pinocchio_token_2022::state::TokenAccount;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive()]
pub struct Mint(pinocchio_token_2022::state::Mint);

impl anchor_lang::AccountDeserialize for Mint {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        let mint_ref = unsafe { pinocchio_token_2022::state::Mint::from_bytes_unchecked(buf) };
        let mint = unsafe { std::ptr::read(mint_ref) };
        Ok(Mint(mint))
    }
}

impl anchor_lang::AccountSerialize for Mint {}

impl anchor_lang::Owners for Mint {
    fn owners() -> &'static [Pubkey] {
        &IDS
    }
}

impl Deref for Mint {
    type Target = pinocchio_token_2022::state::Mint;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct TokenInterface;

impl anchor_lang::Ids for TokenInterface {
    fn ids() -> &'static [Pubkey] {
        &IDS
    }
}

pub type ExtensionsVec = Vec<ExtensionType>;

pub fn find_mint_account_size(extensions: Option<&ExtensionsVec>) -> anchor_lang::Result<usize> {
    if let Some(extensions) = extensions {
        Ok(todo!())
    } else {
        Ok(todo!())
    }
}

pub fn get_mint_extension_data<T: Extension + Pod>(
    account: &AccountInfo,
) -> anchor_lang::Result<T> {
    let mint_data = unsafe { account.borrow_unchecked() };
    let mint_with_extension = todo!();
    let extension_data = *mint_with_extension.get_extension::<T>()?;
    Ok(extension_data)
}
