use anchor_lang::__private::bytemuck::Pod;
use anchor_lang::pinocchio_runtime::program_pack::Pack;
use anchor_lang::pinocchio_runtime::pubkey::Pubkey;
use anchor_lang::pinocchio_runtime::account_info::AccountInfo;
use pinocchio_token_2022::extension::ExtensionType;
use pinocchio_token_2022::extension::{BaseStateWithExtensions, Extension, StateWithExtensions};
use std::ops::Deref;

pub use crate::token_2022::*;
#[cfg(feature = "token_2022_extensions")]
pub use crate::token_2022_extensions::*;

static IDS: [Pubkey; 2] = [pinocchio_token::ID, pinocchio_token_2022::ID];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TokenAccount(pinocchio_token_2022::state::TokenAccount);

impl anchor_lang::AccountDeserialize for TokenAccount {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        pinocchio_token_2022::extension::StateWithExtensions::<pinocchio_token_2022::state::TokenAccount>::unpack(
            buf,
        )
        .map(|t| TokenAccount(t.base))
        .map_err(Into::into)
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mint(pinocchio_token_2022::state::Mint);

impl anchor_lang::AccountDeserialize for Mint {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        pinocchio_token_2022::extension::StateWithExtensions::<pinocchio_token_2022::state::Mint>::unpack(buf)
            .map(|t| Mint(t.base))
            .map_err(Into::into)
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
        Ok(ExtensionType::try_calculate_account_len::<
            pinocchio_token_2022::state::Mint,
        >(extensions)?)
    } else {
        Ok(pinocchio_token_2022::state::Mint::LEN)
    }
}

pub fn get_mint_extension_data<T: Extension + Pod>(
    account: &AccountInfo,
) -> anchor_lang::Result<T> {
    let mint_data = account.data.borrow();
    let mint_with_extension =
        StateWithExtensions::<pinocchio_token_2022::state::Mint>::unpack(&mint_data)?;
    let extension_data = *mint_with_extension.get_extension::<T>()?;
    Ok(extension_data)
}
