use solana_sdk::account_info::AccountInfo;
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use std::ops::{Deref, DerefMut};

pub use anchor_attributes_access_control::access_control;
pub use anchor_attributes_program::program;
pub use anchor_derive::Accounts;
pub use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

pub struct ProgramAccount<'a, T: AnchorSerialize + AnchorDeserialize> {
    pub info: AccountInfo<'a>,
    pub account: T,
}

impl<'a, T: AnchorSerialize + AnchorDeserialize> ProgramAccount<'a, T> {
    pub fn new(info: AccountInfo<'a>, account: T) -> ProgramAccount<'a, T> {
        Self { info, account }
    }
}

impl<'a, T: AnchorSerialize + AnchorDeserialize> Deref for ProgramAccount<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

impl<'a, T: AnchorSerialize + AnchorDeserialize> DerefMut for ProgramAccount<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account
    }
}

pub trait Accounts<'info>: Sized {
    fn try_anchor(program_id: &Pubkey, from: &[AccountInfo<'info>]) -> Result<Self, ProgramError>;
}

pub struct Context<'a, 'b, T> {
    pub accounts: &'a mut T,
    pub program_id: &'b Pubkey,
}

pub mod prelude {
    pub use super::{
        access_control, program, Accounts, AnchorDeserialize, AnchorSerialize, Context,
        ProgramAccount,
    };

    pub use solana_program::msg;
    pub use solana_sdk::account_info::next_account_info;
    pub use solana_sdk::account_info::AccountInfo;
    pub use solana_sdk::entrypoint::ProgramResult;
    pub use solana_sdk::program_error::ProgramError;
    pub use solana_sdk::pubkey::Pubkey;
}
