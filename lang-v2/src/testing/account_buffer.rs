//! Mock `AccountView` scaffold for anchor-lang-v2 integration tests.
//!
//! Construct a stack-backed `AccountView` instance without running under the
//! SVM loader. Enables Miri Tree Borrows witnesses for the aliasing patterns
//! anchor-v2 relies on (typed `CpiHandle` + unchecked CPI, `AccountView:
//! Copy` shared state, `Slab::header_ptr` write provenance).
//!
//! ## Usage
//!
//! ```ignore
//! #[path = "common/account_buffer.rs"]
//! mod account_buffer;
//! use account_buffer::AccountBuffer;
//!
//! let mut buf = AccountBuffer::<256>::new();
//! buf.init([1; 32], [0; 32], /*data_len*/ 0, /*is_signer*/ true,
//!          /*is_writable*/ false, /*executable*/ false);
//! let view = unsafe { buf.view() };
//! // Use `view` in tests — e.g. Miri soundness witnesses.
//! ```

use pinocchio::account::{AccountView, RuntimeAccount};
use solana_address::Address;

/// Size of the RuntimeAccount header + minimum 8 bytes for data/padding.
pub const MIN_ACCOUNT_BUF: usize = core::mem::size_of::<RuntimeAccount>() + 8;

/// Stack-allocated account buffer. `N` is total buffer size in bytes.
/// Header occupies `size_of::<RuntimeAccount>()` bytes; remainder is
/// available for account data (bounded by `data_len` set in `init`).
///
/// `#[repr(C, align(8))]` matches `RuntimeAccount`'s 8-byte alignment
/// requirement.
#[repr(C, align(8))]
pub struct AccountBuffer<const N: usize> {
    inner: [u8; N],
}

impl<const N: usize> AccountBuffer<N> {
    pub fn new() -> Self {
        assert!(
            N >= core::mem::size_of::<RuntimeAccount>(),
            "AccountBuffer<N> needs N >= size_of::<RuntimeAccount>()"
        );
        Self { inner: [0u8; N] }
    }

    /// Raw pointer to the header region.
    pub fn raw(&mut self) -> *mut RuntimeAccount {
        self.inner.as_mut_ptr() as *mut RuntimeAccount
    }

    /// Populate the header. `NOT_BORROWED` = 255 (= `NON_DUP_MARKER`)
    /// means the account is ready for mut/immut borrows.
    pub fn init(
        &mut self,
        address: [u8; 32],
        owner: [u8; 32],
        data_len: usize,
        is_signer: bool,
        is_writable: bool,
        executable: bool,
    ) {
        let raw = self.raw();
        // SAFETY: raw points at a zero-initialized buffer of size N >=
        // size_of::<RuntimeAccount>(), aligned to 8.
        unsafe {
            (*raw).borrow_state = pinocchio::account::NOT_BORROWED;
            (*raw).is_signer = is_signer as u8;
            (*raw).is_writable = is_writable as u8;
            (*raw).executable = executable as u8;
            (*raw).padding = [0u8; 4];
            (*raw).address = Address::new_from_array(address);
            (*raw).owner = Address::new_from_array(owner);
            (*raw).lamports = 100;
            (*raw).data_len = data_len as u64;
        }
    }

    /// Set the account's data bytes (at offset `size_of::<RuntimeAccount>()`
    /// through `+ data_len`). Caller must ensure `init` was called with a
    /// matching `data_len`.
    pub fn write_data(&mut self, data: &[u8]) {
        let offset = core::mem::size_of::<RuntimeAccount>();
        assert!(
            offset + data.len() <= N,
            "write_data would overflow buffer: offset {} + data.len() {} > N {}",
            offset,
            data.len(),
            N
        );
        self.inner[offset..offset + data.len()].copy_from_slice(data);
    }

    /// Read the data region as a byte slice (bounded by data_len in header).
    pub fn read_data(&self) -> &[u8] {
        let offset = core::mem::size_of::<RuntimeAccount>();
        // Cast raw pointer to read data_len (can't call AccountView method
        // without an AccountView).
        let raw = self.inner.as_ptr() as *const RuntimeAccount;
        let data_len = unsafe { (*raw).data_len as usize };
        assert!(offset + data_len <= N, "data_len exceeds buffer");
        &self.inner[offset..offset + data_len]
    }

    /// Construct an `AccountView` over this buffer. The buffer must
    /// outlive the view.
    ///
    /// # Safety
    ///
    /// Caller must ensure `init()` was called. The returned `AccountView`
    /// borrows the buffer via a raw pointer — do not drop or move the
    /// `AccountBuffer` while the `AccountView` is live.
    pub unsafe fn view(&mut self) -> AccountView {
        AccountView::new_unchecked(self.raw())
    }

    /// Direct access to the borrow state byte. Useful for setting up
    /// duplicate-account scenarios where `borrow_state` encodes a dup
    /// index (0..=254) instead of `NOT_BORROWED` (255).
    pub fn set_borrow_state(&mut self, value: u8) {
        unsafe {
            (*self.raw()).borrow_state = value;
        }
    }

    /// Direct access to the lamports field.
    pub fn set_lamports(&mut self, value: u64) {
        unsafe {
            (*self.raw()).lamports = value;
        }
    }

    /// Overwrite the `data_len` field in the header. Useful for
    /// exercising post-construction resize scenarios without going
    /// through a full CPI path.
    pub fn set_data_len(&mut self, value: u64) {
        unsafe {
            (*self.raw()).data_len = value;
        }
    }

    /// Overwrite the `owner` field. Useful for simulating a CPI that
    /// transfers ownership of the account.
    pub fn set_owner(&mut self, owner: [u8; 32]) {
        unsafe {
            (*self.raw()).owner = Address::new_from_array(owner);
        }
    }
}
