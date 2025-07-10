use std::{
    alloc::Layout,
    cell::RefCell,
    mem::size_of,
    ptr::null_mut,
    rc::Rc,
    slice::{from_raw_parts, from_raw_parts_mut},
};
extern crate alloc;
use crate::{
    account::AccountInfo, log::sol_log_64, program_error::ProgramError, pubkey::Pubkey,
    utxo::UtxoMeta,
};
use alloc::vec::Vec;
/// Start address of the memory region used for program heap.
pub const HEAP_START_ADDRESS: u64 = 0x300000000;
/// Length of the heap memory region used for program heap.
pub const HEAP_LENGTH: usize = 32 * 1024;
/// Maximum permitted size of account data (10 MiB).
pub const MAX_PERMITTED_DATA_LENGTH: usize = 10 * 1024 * 1024;
/// Maximum number of bytes a program may add to an account during a single realloc
pub const MAX_PERMITTED_DATA_INCREASE: usize = 10 * 1_024;

pub const BPF_ALIGN_OF_U128: usize = 8;

/// Maximum number of instruction utxos that can be serialized into the
/// SBF VM.
pub const NON_DUP_MARKER: u8 = u8::MAX;

/// type returned by a on-chain program
pub type ProgramResult = Result<(), ProgramError>;

/// Programs indicate success with a return value of 0
pub const SUCCESS: u64 = 0;

/// User implemented function to process an instruction
///
/// program_id: Program ID of the currently executing program accounts: Accounts
/// passed as part of the instruction instruction_data: Instruction data
pub type ProcessInstruction =
    fn(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult;

/// The bump allocator used as the default rust heap when running programs.
pub struct BumpAllocator {
    pub start: usize,
    pub len: usize,
}
/// Integer arithmetic in this global allocator implementation is safe when
/// operating on the prescribed `HEAP_START_ADDRESS` and `HEAP_LENGTH`. Any
/// other use may overflow and is thus unsupported and at one's own risk.
#[allow(clippy::arithmetic_side_effects)]
unsafe impl std::alloc::GlobalAlloc for BumpAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pos_ptr = self.start as *mut usize;

        let cur_pos = *pos_ptr;
        let cur_heap_used = self.start + self.len - cur_pos;

        sol_log_64(layout.size() as u64, cur_heap_used as u64, 0, 0, 0);

        let mut pos = *pos_ptr;
        if pos == 0 {
            // First time, set starting position
            pos = self.start + self.len;
        }
        pos = pos.saturating_sub(layout.size());
        pos &= !(layout.align().wrapping_sub(1));
        if pos < self.start + size_of::<*mut u8>() {
            return null_mut();
        }
        *pos_ptr = pos;
        pos as *mut u8
    }
    #[inline]
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        // I'm a bump allocator, I don't free
    }
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn deserialize<'a>(input: *mut u8) -> (&'a Pubkey, Vec<AccountInfo<'a>>, &'a [u8]) {
    let mut offset: usize = 0;

    #[allow(clippy::cast_ptr_alignment)]
    let num_accounts = *(input.add(offset) as *const u64) as usize;
    offset += size_of::<u64>();

    let mut accounts = Vec::with_capacity(num_accounts);

    for _ in 0..num_accounts {
        let dup_info = *(input.add(offset) as *const u8);
        offset += size_of::<u8>();
        if dup_info == NON_DUP_MARKER {
            offset += 4 * size_of::<u8>();

            let is_signer = *(input.add(offset) as *const bool);
            offset += size_of::<bool>();

            let is_writable = *(input.add(offset) as *const bool);
            offset += size_of::<bool>();

            let is_executable = *(input.add(offset) as *const bool);
            offset += size_of::<bool>();

            let lamports = Rc::new(RefCell::new(&mut *(input.add(offset) as *mut u64)));
            offset += size_of::<u64>();

            let key: &Pubkey = &*(input.add(offset) as *const Pubkey);
            offset += size_of::<Pubkey>();

            //skipping original data_len
            offset += size_of::<u64>();

            let data_len = *(input.add(offset) as *const u64) as usize;
            offset += size_of::<u64>();

            let data = Rc::new(RefCell::new({
                from_raw_parts_mut(input.add(offset), data_len)
            }));

            offset += data_len + MAX_PERMITTED_DATA_INCREASE;
            offset += (offset as *const u8).align_offset(BPF_ALIGN_OF_U128); // padding

            let owner: &Pubkey = &*(input.add(offset) as *const Pubkey);
            offset += size_of::<Pubkey>();

            let utxo: &UtxoMeta = &*(input.add(offset) as *const UtxoMeta);
            offset += size_of::<UtxoMeta>();

            offset += 4 * size_of::<u8>();

            accounts.push(AccountInfo {
                key,
                lamports,
                utxo,
                data,
                owner,
                is_signer,
                is_writable,
                is_executable,
            });
        } else {
            offset += 7; // padding

            // Duplicate account, clone the original
            accounts.push(accounts[dup_info as usize].clone());
        }
    }

    // Instruction data

    #[allow(clippy::cast_ptr_alignment)]
    let instruction_data_len = *(input.add(offset) as *const u64) as usize;
    offset += size_of::<u64>();

    let instruction_data = { from_raw_parts(input.add(offset), instruction_data_len) };
    offset += instruction_data_len;

    // Program Id

    let program_id: &Pubkey = &*(input.add(offset) as *const Pubkey);

    (program_id, accounts, instruction_data)
}

#[macro_export]
macro_rules! entrypoint {
    ($process_instruction:ident) => {
        /// # Safety
        #[no_mangle]
        pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64 {
            use std::collections::HashMap;
            let (program_id, utxos, instruction_data) =
                unsafe { $crate::entrypoint::deserialize(input) };
            match $process_instruction(&program_id, &utxos, &instruction_data) {
                Ok(()) => $crate::entrypoint::SUCCESS,
                Err(error) => error.into(),
            }
        }
        $crate::custom_heap_default!();
        $crate::custom_panic_default!();
    };
}

#[macro_export]
macro_rules! custom_heap_default {
    () => {
        #[cfg(target_os = "solana")]
        #[global_allocator]
        static A: $crate::entrypoint::BumpAllocator = $crate::entrypoint::BumpAllocator {
            start: $crate::entrypoint::HEAP_START_ADDRESS as usize,
            len: $crate::entrypoint::HEAP_LENGTH,
        };

        // For regular host builds (unit tests, trybuild, etc.) fall back to the system allocator
        // to avoid dereferencing unmapped addresses like `0x300000000`, which leads to
        // segmentation faults during tests.
        #[cfg(not(target_os = "solana"))]
        #[global_allocator]
        static A: std::alloc::System = std::alloc::System;
    };
}

#[macro_export]
macro_rules! custom_panic_default {
    () => {
        #[no_mangle]
        fn custom_panic(info: &core::panic::PanicInfo<'_>) {
            // Full panic reporting
            $crate::msg!("{}", info);
        }
    };
}
