use pinocchio::{account::AccountView, address::Address};

pub struct Context<'a, T> {
    pub program_id: Address,
    pub accounts: T,
    pub remaining_accounts: &'a [AccountView],
}

impl<'a, T> Context<'a, T> {
    pub fn new(program_id: Address, accounts: T, remaining_accounts: &'a [AccountView]) -> Self {
        Self { program_id, accounts, remaining_accounts }
    }
}
