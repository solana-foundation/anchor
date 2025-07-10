//! Implementations of syscalls used when `arch-program` is built for non-SBF targets.

pub const UNIMPLEMENTED: u64 = 0;
use crate::{
    account::AccountInfo, clock::Clock, entrypoint::ProgramResult, instruction::Instruction,
    pubkey::Pubkey, utxo::UtxoMeta,
};

pub(crate) fn sol_log(message: &str) {
    println!("{message}");
}
pub(crate) fn _sol_log_64_(arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) {
    sol_log(&format!("{arg1:?}, {arg2:?},{arg3:?},{arg4:?},{arg5:?}"))
}
pub(crate) fn sol_memset(_s: *mut u8, _c: u8, _n: usize) {
    sol_log("UNAVAILABLE");
}
pub(crate) fn sol_memmove(_dst: *mut u8, _src: *const u8, _n: usize) {
    sol_log("UNAVAILABLE");
}
pub(crate) fn sol_memcpy(_dst: *mut u8, _src: *const u8, _n: usize) {
    sol_log("UNAVAILABLE");
}
pub(crate) fn sol_memcmp(_s1: *const u8, _s2: *const u8, _n: usize, _result: *mut i32) {
    sol_log("UNAVAILABLE");
}
pub(crate) fn _sol_set_return_data(_data: *const u8, _length: u64) {
    sol_log("UNAVAILABLE");
}
pub(crate) fn _sol_log_pubkey(_pubkey_addr: *const u8) {
    sol_log("UNAVAILABLE");
}
pub(crate) fn _sol_log_data(_data: *const u8, _data_len: u64) {
    sol_log("UNAVAILABLE");
}
pub(crate) fn _sol_get_return_data(_data: *mut u8, _length: u64, _program_id: *mut Pubkey) -> u64 {
    sol_log("UNAVAILABLE");
    UNIMPLEMENTED
}
pub(crate) fn arch_set_transaction_to_sign(_transaction_to_sign: *const u8, _length: usize) -> u64 {
    sol_log("UNAVAILABLE");
    UNIMPLEMENTED
}
pub(crate) fn arch_get_bitcoin_tx(_buf: *const u8, _buf_len: usize, _txid: &[u8; 32]) -> u64 {
    sol_log("UNAVAILABLE");
    UNIMPLEMENTED
}
pub(crate) fn arch_get_runes_from_output(
    _buf: *const u8,
    _buf_len: usize,
    _txid: &[u8; 32],
    _output_index: u32,
) -> u64 {
    sol_log("UNAVAILABLE");
    UNIMPLEMENTED
}
pub(crate) fn arch_get_network_xonly_pubkey(_data: *mut u8) -> u64 {
    sol_log("UNAVAILABLE");
    UNIMPLEMENTED
}
pub(crate) fn arch_validate_utxo_ownership(_utxo: *const UtxoMeta, _owner: *const Pubkey) -> u64 {
    sol_log("UNAVAILABLE");
    UNIMPLEMENTED
}
pub(crate) fn arch_get_account_script_pubkey(_buf: &mut [u8; 34], _pubkey: &Pubkey) {}

pub(crate) fn sol_invoke_signed(
    _instruction_addr: &Instruction,
    _account_infos: &[AccountInfo],
    _signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    sol_log("SyscallStubs: sol_invoke_signed() not available");
    Ok(())
}

pub(crate) fn sol_secp256k1_recover(
    _hash_addr: *const u8,
    _recovery_id_val: u64,
    _signature_addr: *const u8,
    _result_addr: *mut u8,
) -> u64 {
    sol_log("UNAVAILABLE");
    UNIMPLEMENTED
}

pub(crate) fn sol_log_compute_units() {
    sol_log("UNAVAILABLE");
}

pub(crate) fn get_remaining_compute_units() -> u64 {
    sol_log("UNAVAILABLE");
    UNIMPLEMENTED
}

pub(crate) fn arch_get_bitcoin_block_height() -> u64 {
    UNIMPLEMENTED
}

pub(crate) fn arch_get_clock(_clock: *mut Clock) -> u64 {
    UNIMPLEMENTED
}
