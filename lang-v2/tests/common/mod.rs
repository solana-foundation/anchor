#![allow(dead_code)]  // helpers used by a subset of integration test files

//! Shared test scaffolding for anchor-lang-v2 integration tests.
//!
//! Construct mock `AccountView` instances without running under the SVM
//! loader. Enables Miri Tree Borrows witnesses for the aliasing patterns
//! anchor-v2 relies on (typed `CpiHandle` + unchecked CPI, `AccountView:
//! Copy` shared state, `Slab::header_ptr` write provenance).
//!
//! ## Usage
//!
//! ```ignore
//! use common::AccountBuffer;
//! let mut buf = AccountBuffer::<256>::new();
//! buf.init([1; 32], [0; 32], /*data_len*/ 0, /*is_signer*/ true,
//!          /*is_writable*/ false, /*executable*/ false);
//! let view = unsafe { buf.view() };
//! // Use `view` in tests — e.g. Miri soundness witnesses.
//! ```
//!
//! ## Extraction plan
//!
//! If this scaffold proves durable, promote it to a `testing` feature
//! flag on `anchor-lang-v2` so downstream crates can build against it
//! without duplicating. Today it's scoped to this crate's integration
//! tests only.

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
}

// ---------------------------------------------------------------------------
// SBF Serialized-Input Buffer
//
// Models the exact layout that the Solana BPF loader writes into a
// program's r1 register at entrypoint, for cursor-walk tests.
//
// Layout:
//   num_accounts: u64 (8 bytes, LE)
//   per-account record:
//     if non-dup:
//       RuntimeAccount header (88 bytes, borrow_state = NON_DUP_MARKER = 255)
//       account data (data_len bytes)
//       padding (MAX_PERMITTED_DATA_INCREASE = 10,240 bytes)
//       rent_epoch (8 bytes)
//       alignment padding to 8-byte boundary
//     if dup:
//       dup_marker (1 byte, value = index of earlier account)
//       padding (7 bytes) to round to 8-byte alignment
//   instruction data
//   program_id (32 bytes)
// ---------------------------------------------------------------------------

use pinocchio::account::MAX_PERMITTED_DATA_INCREASE;

/// Serialized-input buffer simulating what the SBF loader writes.
///
/// **Alignment matters.** The real SBF loader aligns the input buffer
/// to 8 bytes (u64 alignment) because `RuntimeAccount` requires it
/// (`cursor.rs:136` does `*account.borrow_state` through
/// `*mut RuntimeAccount`, and Rust's `*mut T` dereference requires
/// `T`'s alignment).
///
/// `Vec<u8>` gives only u8 alignment. So we back with `Vec<u64>` and
/// expose the bytes via raw pointer cast. `Vec<u64>` guarantees at
/// least 8-byte alignment from the allocator.
pub struct SbfInputBuffer {
    // Backing store — 8-byte-aligned because element type is u64.
    backing: Vec<u64>,
    // Logical byte length (may be less than backing.len() * 8).
    len: usize,
    /// Byte offset where each account record starts. Useful for
    /// cursor-walk tests that need to reason about positions.
    pub record_offsets: Vec<usize>,
}

#[derive(Clone, Copy)]
pub enum AccountRecord {
    /// Non-duplicate account with the given header + data.
    NonDup {
        address: [u8; 32],
        owner: [u8; 32],
        lamports: u64,
        is_signer: bool,
        is_writable: bool,
        executable: bool,
        data_len: usize,
    },
    /// Duplicate of an earlier account at `index`.
    Dup { index: u8 },
}

impl SbfInputBuffer {
    /// Build a serialized input buffer from a list of account records.
    /// Non-dup records zero-fill their data region (matching SVM behavior
    /// for fresh accounts).
    pub fn build(records: &[AccountRecord]) -> Self {
        // Compute total byte length first; collect bytes into a temp
        // Vec<u8>, then move into a 8-aligned Vec<u64> backing.
        let mut bytes: Vec<u8> = Vec::new();
        let mut record_offsets = Vec::with_capacity(records.len());

        bytes.extend_from_slice(&(records.len() as u64).to_le_bytes());

        for record in records {
            while bytes.len() % 8 != 0 {
                bytes.push(0);
            }
            record_offsets.push(bytes.len());

            match *record {
                AccountRecord::NonDup {
                    address,
                    owner,
                    lamports,
                    is_signer,
                    is_writable,
                    executable,
                    data_len,
                } => {
                    bytes.push(pinocchio::account::NOT_BORROWED);
                    bytes.push(is_signer as u8);
                    bytes.push(is_writable as u8);
                    bytes.push(executable as u8);
                    bytes.extend_from_slice(&[0u8; 4]);
                    bytes.extend_from_slice(&address);
                    bytes.extend_from_slice(&owner);
                    bytes.extend_from_slice(&lamports.to_le_bytes());
                    bytes.extend_from_slice(&(data_len as u64).to_le_bytes());
                    bytes.extend(core::iter::repeat_n(0u8, data_len));
                    bytes.extend(core::iter::repeat_n(0u8, MAX_PERMITTED_DATA_INCREASE));
                    bytes.extend_from_slice(&0u64.to_le_bytes());
                }
                AccountRecord::Dup { index } => {
                    bytes.push(index);
                    bytes.extend_from_slice(&[0u8; 7]);
                }
            }
        }

        let len = bytes.len();

        // Transfer into 8-byte-aligned Vec<u64>. Round up length.
        let num_u64s = len.div_ceil(8);
        let mut backing: Vec<u64> = vec![0u64; num_u64s];
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                backing.as_mut_ptr() as *mut u8,
                len,
            );
        }

        Self { backing, len, record_offsets }
    }

    /// Pointer to the start of the buffer (the `num_accounts` prefix).
    /// Guaranteed 8-byte aligned (backing is `Vec<u64>`).
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.backing.as_mut_ptr() as *mut u8
    }

    /// Number of account records declared in the prefix.
    pub fn num_accounts(&self) -> u64 {
        let first_u64 = self.backing[0];
        u64::from_le(first_u64)
    }

    /// Access the raw bytes as a mutable slice.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self.backing.as_mut_ptr() as *mut u8, self.len)
        }
    }
}

