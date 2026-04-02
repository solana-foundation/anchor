//! Build a valid `RuntimeAccount` + trailing data buffer for `AccountView::new_unchecked`.

use {
    anchor_lang::{
        pinocchio_runtime::account::{RuntimeAccount, NOT_BORROWED},
        prelude::Pubkey,
        AccountView,
    },
    std::{mem::size_of, ptr},
};

/// Owns the backing allocation for a single `AccountView`.
pub struct OwnedPinocchioAccount {
    #[allow(dead_code)]
    storage: Box<[u8]>,
    pub info: AccountView,
}

impl OwnedPinocchioAccount {
    pub fn new(key: Pubkey, owner: Pubkey, lamports: u64, data: &[u8]) -> Self {
        let header_len = size_of::<RuntimeAccount>();
        let total = header_len + data.len();
        let mut storage = vec![0u8; total].into_boxed_slice();
        let header_ptr = storage.as_mut_ptr().cast::<RuntimeAccount>();
        let acc = RuntimeAccount {
            borrow_state: NOT_BORROWED,
            is_signer: 0,
            is_writable: 1,
            executable: 0,
            padding: [0; 4],
            address: key,
            owner,
            lamports,
            data_len: data.len() as u64,
        };
        unsafe {
            ptr::write(header_ptr, acc);
            ptr::copy_nonoverlapping(
                data.as_ptr(),
                storage.as_mut_ptr().add(header_len),
                data.len(),
            );
        }
        let info = unsafe { AccountView::new_unchecked(header_ptr) };
        Self { storage, info }
    }

    /// Mutate the serialized owner field (used to simulate CPI changing ownership).
    // `account_reload` uses this; other integration test crates pull in this module too.
    #[allow(dead_code)]
    pub fn set_owner(&mut self, new_owner: Pubkey) {
        let header = self.storage.as_mut_ptr().cast::<RuntimeAccount>();
        unsafe {
            (*header).owner = new_owner;
        }
    }
}
