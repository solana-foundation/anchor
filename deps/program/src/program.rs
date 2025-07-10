use bitcoin::hashes::Hash;
use bitcoin::Transaction;
use bitcoin_slices::{bsl, Error, Visit, Visitor};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::input_to_sign::InputToSign;
use crate::instruction::Instruction;
use crate::program_error::ProgramError;
use crate::rune::RuneAmount;
#[cfg(target_os = "solana")]
use crate::stable_layout::stable_ins::StableInstruction;
use crate::{msg, MAX_BTC_RUNE_OUTPUT_SIZE, MAX_BTC_TX_SIZE};

use crate::clock::Clock;
use crate::transaction_to_sign::TransactionToSign;
use crate::utxo::UtxoMeta;
use crate::{account::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

/// A generic wrapper for fixed-size data that avoids heap allocation.
///
/// This type holds raw bytes in a fixed-size array and tracks the actual size of the data.
/// The array size is determined by the const generic parameter `N`.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FixedSizeBuffer<const N: usize> {
    data: [u8; N],
    size: usize,
}

impl<const N: usize> FixedSizeBuffer<N> {
    /// Creates a new FixedSizeBuffer from a buffer and size.
    pub fn new(data: [u8; N], size: usize) -> Self {
        Self { data, size }
    }

    /// Returns the actual size of the data.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns a slice of the actual data.
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.size]
    }

    /// Returns a mutable raw pointer to the underlying buffer (for FFI/syscall writes).
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }

    /// Returns the total capacity of the buffer.
    pub fn capacity(&self) -> usize {
        N
    }

    /// Sets the length of the valid data written into the buffer.
    ///
    /// # Safety
    /// The caller must guarantee that `new_size` bytes starting from the
    /// pointer returned by `as_mut_ptr` have been initialised.
    pub fn set_size(&mut self, new_size: usize) {
        debug_assert!(
            new_size <= N,
            "new_size ({}) exceeds buffer capacity ({})",
            new_size,
            N
        );

        self.size = new_size;
    }
}

impl<const N: usize> AsRef<[u8]> for FixedSizeBuffer<N> {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<const N: usize> std::ops::Deref for FixedSizeBuffer<N> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<const N: usize> Default for FixedSizeBuffer<N> {
    fn default() -> Self {
        Self {
            data: [0u8; N],
            size: 0,
        }
    }
}

/// Type alias for Bitcoin transaction data with a fixed 3976-byte buffer.
pub type BitcoinTransaction = FixedSizeBuffer<MAX_BTC_TX_SIZE>;

/// Type alias for Bitcoin rune output data with a fixed 2048-byte buffer.
pub type BitcoinRuneOutput = FixedSizeBuffer<MAX_BTC_RUNE_OUTPUT_SIZE>;

/// Type alias for Returned Data with a fixed 1024-byte buffer.
pub type ReturnedData = FixedSizeBuffer<MAX_RETURN_DATA>;

/// Invokes a program instruction through cross-program invocation.
///
/// This function processes the provided instruction by dispatching control to another program
/// using the account information provided.
///
/// # Arguments
/// * `instruction` - The instruction to process
/// * `account_infos` - The accounts required to process the instruction
///
/// # Returns
/// * `ProgramResult` - Ok(()) if successful, or an error if the operation fails
pub fn invoke(instruction: &Instruction, account_infos: &[AccountInfo]) -> ProgramResult {
    invoke_signed(instruction, account_infos, &[])
}

/// Invokes a program instruction without checking account permissions.
///
/// Similar to `invoke`, but skips the account permission checking step.
/// This is generally less safe than `invoke` and should be used carefully.
///
/// # Arguments
/// * `instruction` - The instruction to process
/// * `account_infos` - The accounts required to process the instruction
///
/// # Returns
/// * `ProgramResult` - Ok(()) if successful, or an error if the operation fails
pub fn invoke_unchecked(instruction: &Instruction, account_infos: &[AccountInfo]) -> ProgramResult {
    invoke_signed_unchecked(instruction, account_infos, &[])
}

/// Invokes a program instruction with additional signing authority.
///
/// This function processes the provided instruction by dispatching control to another program,
/// while also providing program-derived address signing authority.
/// It performs permission checks on the accounts before invoking.
///
/// # Arguments
/// * `instruction` - The instruction to process
/// * `account_infos` - The accounts required to process the instruction
/// * `signers_seeds` - Seeds used to sign the transaction as a program-derived address
///
/// # Returns
/// * `ProgramResult` - Ok(()) if successful, or an error if the operation fails
///
/// # Errors
/// Returns an error if any required account cannot be borrowed according to its stated permissions
pub fn invoke_signed(
    instruction: &Instruction,
    account_infos: &[AccountInfo],
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    // Check that the account RefCells are consistent with the request
    for account_meta in instruction.accounts.iter() {
        for account_info in account_infos.iter() {
            if account_meta.pubkey == *account_info.key {
                if account_meta.is_writable {
                    let _ = account_info.try_borrow_mut_data()?;
                } else {
                    let _ = account_info.try_borrow_data()?;
                }
                break;
            }
        }
    }

    invoke_signed_unchecked(instruction, account_infos, signers_seeds)
}

/// Invokes a program instruction with additional signing authority without checking account permissions.
///
/// Similar to `invoke_signed`, but skips the account permission checking step.
/// This is generally less safe than `invoke_signed` and should be used carefully.
///
/// # Arguments
/// * `instruction` - The instruction to process
/// * `account_infos` - The accounts required to process the instruction
/// * `signers_seeds` - Seeds used to sign the transaction as a program-derived address
///
/// # Returns
/// * `ProgramResult` - Ok(()) if successful, or an error if the operation fails
pub fn invoke_signed_unchecked(
    instruction: &Instruction,
    account_infos: &[AccountInfo],
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    #[cfg(target_os = "solana")]
    {
        let instruction = StableInstruction::from(instruction.clone());
        let result = unsafe {
            crate::syscalls::sol_invoke_signed_rust(
                &instruction as *const _ as *const u8,
                account_infos as *const _ as *const u8,
                account_infos.len() as u64,
                signers_seeds as *const _ as *const u8,
                signers_seeds.len() as u64,
            )
        };
        match result {
            crate::entrypoint::SUCCESS => Ok(()),
            _ => Err(result.into()),
        }
    }

    #[cfg(not(target_os = "solana"))]
    crate::program_stubs::sol_invoke_signed(instruction, account_infos, signers_seeds)
}

/// Gets the next account from an account iterator.
///
/// A utility function that advances the iterator and returns the next `AccountInfo`,
/// or returns a `NotEnoughAccountKeys` error if there are no more accounts.
///
/// # Arguments
/// * `iter` - Mutable reference to an iterator yielding references to `AccountInfo`
///
/// # Returns
/// * `Result<&AccountInfo, ProgramError>` - The next account info or an error if depleted
///
/// # Errors
/// Returns `ProgramError::NotEnoughAccountKeys` if the iterator has no more items
pub fn next_account_info<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
    iter: &mut I,
) -> Result<I::Item, ProgramError> {
    iter.next().ok_or(ProgramError::NotEnoughAccountKeys)
}

pub const MAX_TRANSACTION_TO_SIGN: usize = 4 * 1024;

/// Sets an Arch transaction to be signed by the program.
///
/// This function takes a transaction and its associated signing metadata and prepares it
/// for signing through the runtime. It also updates the UTXO metadata for relevant accounts.
///
/// # Arguments
/// * `accounts` - Slice of account information required for the transaction
/// * `tx` - The transaction
/// * `inputs_to_sign` - The inputs to sign
///
/// # Returns
/// * `ProgramResult` - Ok(()) if successful, or an error if the operation fails
pub fn set_transaction_to_sign<'accounts, 'info, T>(
    accounts: &'accounts mut [T],
    tx: &'accounts Transaction,
    inputs_to_sign: &'accounts [InputToSign],
) -> ProgramResult
where
    T: AsRef<AccountInfo<'info>>,
{
    msg!("setting tx to sign");
    // Use the new method that avoids double allocation
    let serialized_transaction_to_sign = TransactionToSign::serialise_with_tx(tx, inputs_to_sign);

    #[cfg(target_os = "solana")]
    let result = unsafe {
        crate::syscalls::arch_set_transaction_to_sign(
            serialized_transaction_to_sign.as_ptr(),
            serialized_transaction_to_sign.len() as u64,
        )
    };
    #[cfg(not(target_os = "solana"))]
    let result = crate::program_stubs::arch_set_transaction_to_sign(
        serialized_transaction_to_sign.as_ptr(),
        serialized_transaction_to_sign.len(),
    );

    match result {
        crate::entrypoint::SUCCESS => {
            let txid = tx.compute_txid();
            let mut txid_bytes: [u8; 32] = txid.as_raw_hash().to_byte_array();
            txid_bytes.reverse();

            for input in inputs_to_sign {
                if let Some(account) = accounts
                    .iter_mut()
                    .find(|account| *account.as_ref().key == input.signer)
                {
                    account
                        .as_ref()
                        .set_utxo(&UtxoMeta::from(txid_bytes, input.index));
                }
            }
            Ok(())
        }
        _ => Err(result.into()),
    }
}

/// Maximum size that can be set using [`set_return_data`].
pub const MAX_RETURN_DATA: usize = 1024;

/// Set the running program's return data.
///
/// Return data is a dedicated per-transaction buffer for data passed
/// from cross-program invoked programs back to their caller.
///
/// The maximum size of return data is [`MAX_RETURN_DATA`]. Return data is
/// retrieved by the caller with [`get_return_data`].
pub fn set_return_data(data: &[u8]) {
    unsafe { crate::syscalls::sol_set_return_data(data.as_ptr(), data.len() as u64) };
}

/// Get the return data from an invoked program.
///
/// For every transaction there is a single buffer with maximum length
/// [`MAX_RETURN_DATA`], paired with a [`Pubkey`] representing the program ID of
/// the program that most recently set the return data. Thus the return data is
/// a global resource and care must be taken to ensure that it represents what
/// is expected: called programs are free to set or not set the return data; and
/// the return data may represent values set by programs multiple calls down the
/// call stack, depending on the circumstances of transaction execution.
///
/// Return data is set by the callee with [`set_return_data`].
///
/// Return data is cleared before every CPI invocation &mdash; a program that
/// has invoked no other programs can expect the return data to be `None`; if no
/// return data was set by the previous CPI invocation, then this function
/// returns `None`.
///
/// Return data is not cleared after returning from CPI invocations &mdash; a
/// program that has called another program may retrieve return data that was
/// not set by the called program, but instead set by a program further down the
/// call stack; or, if a program calls itself recursively, it is possible that
/// the return data was not set by the immediate call to that program, but by a
/// subsequent recursive call to that program. Likewise, an external RPC caller
/// may see return data that was not set by the program it is directly calling,
/// but by a program that program called.
///
/// For more about return data see the [documentation for the return data proposal][rdp].
///
/// [rdp]: https://docs.solanalabs.com/proposals/return-data
#[inline(never)]
pub fn get_return_data() -> Option<(Pubkey, ReturnedData)> {
    use std::cmp::min;

    let mut buf = [0u8; MAX_RETURN_DATA];
    let mut program_id = Pubkey::default();

    let size = unsafe {
        crate::syscalls::sol_get_return_data(buf.as_mut_ptr(), buf.len() as u64, &mut program_id)
    };

    if size == 0 {
        None
    } else {
        let size = min(size as usize, MAX_RETURN_DATA);
        Some((program_id, ReturnedData::new(buf, size)))
    }
}

/// Retrieves a Bitcoin transaction by its transaction ID.
///
/// # Arguments
/// * `txid` - 32-byte array containing the Bitcoin transaction ID
///
/// # Returns
/// * `Option<BitcoinTransaction>` - The transaction if found, None if not found
#[inline(never)]
pub fn get_bitcoin_tx(txid: [u8; 32]) -> Option<BitcoinTransaction> {
    let mut buf: BitcoinTransaction = Default::default();

    #[cfg(target_os = "solana")]
    let size = unsafe {
        crate::syscalls::arch_get_bitcoin_tx(buf.as_mut_ptr(), buf.capacity() as u64, &txid)
    };
    #[cfg(not(target_os = "solana"))]
    let size = crate::program_stubs::arch_get_bitcoin_tx(buf.as_mut_ptr(), buf.capacity(), &txid);

    if size == 0 {
        return None;
    }

    buf.set_size(core::cmp::min(size as usize, MAX_BTC_TX_SIZE));

    Some(buf)
}

/// Extracts the value of a specific output from a serialized Bitcoin transaction.
///
/// This function is used to extract the value of a specific output from a serialized Bitcoin transaction.
///
/// # Arguments
/// * `tx` - The transaction bytes
/// * `output_index` - The output index to retrieve
///
/// # Returns
/// * `Option<u64>` - The output value if found, None if not found
#[inline(never)]
pub fn get_bitcoin_tx_output_value(txid: [u8; 32], vout: u32) -> Option<u64> {
    let mut buf: BitcoinTransaction = Default::default();

    #[cfg(target_os = "solana")]
    let size = unsafe {
        crate::syscalls::arch_get_bitcoin_tx(buf.as_mut_ptr(), buf.capacity() as u64, &txid)
    };
    #[cfg(not(target_os = "solana"))]
    let size = crate::program_stubs::arch_get_bitcoin_tx(buf.as_mut_ptr(), buf.capacity(), &txid);

    if size == 0 {
        return None;
    }

    buf.set_size(core::cmp::min(size as usize, MAX_BTC_TX_SIZE));

    extract_output_value(buf.as_slice(), vout as usize)
}

#[inline(never)]
fn extract_output_value(tx: &[u8], output_index: usize) -> Option<u64> {
    struct OutputExtractor {
        target_index: usize,
        value: Option<u64>,
    }

    impl Visitor for OutputExtractor {
        fn visit_tx_out(&mut self, vout: usize, tx_out: &bsl::TxOut) -> core::ops::ControlFlow<()> {
            if vout == self.target_index {
                // Calculate the position within the original transaction bytes
                let value = tx_out.value();
                self.value = Some(value);
                return core::ops::ControlFlow::Break(());
            }
            core::ops::ControlFlow::Continue(())
        }
    }

    let mut extractor = OutputExtractor {
        target_index: output_index,
        value: None,
    };

    // Parse transaction and visit outputs
    match bsl::Transaction::visit(tx, &mut extractor) {
        Ok(_) | Err(Error::VisitBreak) => extractor.value,
        Err(_) => None,
    }
}

/// Retrieves the runes from a Bitcoin output by its transaction ID and output index.
///
/// # Arguments
/// * `txid` - 32-byte array containing the Bitcoin transaction ID
/// * `output_index` - The output index to retrieve
///
/// # Returns
/// * `Option<Vec<RuneAmount>>` - The runes if found, None if not found
#[inline(never)]
pub fn get_runes_from_output(txid: [u8; 32], output_index: u32) -> Option<Vec<RuneAmount>> {
    use std::cmp::min;
    if txid == [0u8; 32] {
        return None;
    }

    let mut result: BitcoinRuneOutput = Default::default();

    #[cfg(target_os = "solana")]
    let size = unsafe {
        crate::syscalls::arch_get_runes_from_output(
            result.as_mut_ptr(),
            result.capacity() as u64,
            &txid,
            output_index,
        )
    };

    #[cfg(not(target_os = "solana"))]
    let size = crate::program_stubs::arch_get_runes_from_output(
        result.as_mut_ptr(),
        result.capacity(),
        &txid,
        output_index,
    );

    if size == 0 {
        None
    } else {
        unsafe { result.set_size(min(size as usize, MAX_BTC_RUNE_OUTPUT_SIZE)) };
        borsh::from_slice::<Vec<RuneAmount>>(result.as_slice()).ok()
    }
}

pub fn get_remaining_compute_units() -> u64 {
    #[cfg(target_os = "solana")]
    unsafe {
        crate::syscalls::get_remaining_compute_units()
    }

    #[cfg(not(target_os = "solana"))]
    crate::program_stubs::get_remaining_compute_units()
}
/// Retrieves the network's X-only public key.
///
/// This function fetches the X-only public key associated with the current network configuration.
///
/// # Returns
/// * `[u8; 32]` - The 32-byte X-only public key
pub fn get_network_xonly_pubkey() -> [u8; 32] {
    let mut buf = [0u8; 32];

    #[cfg(target_os = "solana")]
    let _ = unsafe { crate::syscalls::arch_get_network_xonly_pubkey(buf.as_mut_ptr()) };

    #[cfg(not(target_os = "solana"))]
    crate::program_stubs::arch_get_network_xonly_pubkey(buf.as_mut_ptr());
    buf
}

/// Validates if a UTXO is owned by the specified public key.
///
/// # Arguments
/// * `utxo` - The UTXO metadata to validate
/// * `owner` - The public key to check ownership against
///
/// # Returns
/// * `bool` - true if the UTXO is owned by the specified public key, false otherwise
pub fn validate_utxo_ownership(utxo: &UtxoMeta, owner: &Pubkey) -> bool {
    #[cfg(target_os = "solana")]
    unsafe {
        crate::syscalls::arch_validate_utxo_ownership(utxo, owner) != 0
    }

    #[cfg(not(target_os = "solana"))]
    {
        crate::program_stubs::arch_validate_utxo_ownership(utxo, owner) != 0
    }
}

/// Gets the script public key for a given account.
///
/// # Arguments
/// * `pubkey` - The public key of the account
///
/// # Returns
/// * `[u8; 34]` - The 34-byte script public key
pub fn get_account_script_pubkey(pubkey: &Pubkey) -> [u8; 34] {
    let mut buf = [0u8; 34];

    #[cfg(target_os = "solana")]
    let _ = unsafe { crate::syscalls::arch_get_account_script_pubkey(buf.as_mut_ptr(), pubkey) };

    #[cfg(not(target_os = "solana"))]
    crate::program_stubs::arch_get_account_script_pubkey(&mut buf, pubkey);
    buf
}

/// Retrieves the current Bitcoin block height from the runtime.
///
/// # Returns
/// * `u64` - The current Bitcoin block height
pub fn get_bitcoin_block_height() -> u64 {
    #[cfg(target_os = "solana")]
    unsafe {
        crate::syscalls::arch_get_bitcoin_block_height()
    }

    #[cfg(not(target_os = "solana"))]
    crate::program_stubs::arch_get_bitcoin_block_height()
}

/// Gets the current clock information from the runtime.
///
/// # Returns
/// * `Clock` - The current clock state containing timing information
pub fn get_clock() -> Clock {
    let mut clock = Clock::default();
    #[cfg(target_os = "solana")]
    unsafe {
        crate::syscalls::arch_get_clock(&mut clock)
    };

    #[cfg(not(target_os = "solana"))]
    let _ = crate::program_stubs::arch_get_clock(&mut clock);

    clock
}
