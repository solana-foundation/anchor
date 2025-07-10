#![allow(improper_ctypes)]

use crate::{clock::Clock, pubkey::Pubkey, utxo::UtxoMeta};

macro_rules! define_syscall {
	(fn $name:ident($($arg:ident: $typ:ty),*) -> $ret:ty) => {
		extern "C" {
			pub fn $name($($arg: $typ),*) -> $ret;
		}
	};
	(fn $name:ident($($arg:ident: $typ:ty),*)) => {
		define_syscall!(fn $name($($arg: $typ),*) -> ());
	}
}

define_syscall!(fn sol_invoke_signed_rust(instruction_addr: *const u8, account_infos_addr: *const u8, account_infos_len: u64, signers_seeds_addr: *const u8, signers_seeds_len: u64) -> u64);
define_syscall!(fn sol_set_return_data(data: *const u8, length: u64));
define_syscall!(fn sol_get_return_data(data: *mut u8, length: u64, program_id: *mut Pubkey) -> u64);
define_syscall!(fn sol_try_find_program_address(seeds_addr: *const u8, seeds_len: u64, program_id_addr: *const u8, address_bytes_addr: *const u8, bump_seed_addr: *const u8) -> u64);
define_syscall!(fn sol_create_program_address(seeds_addr: *const u8, seeds_len: u64, program_id_addr: *const u8, address_addr: *mut u8) -> u64);

define_syscall!(fn get_remaining_compute_units() -> u64);
define_syscall!(fn arch_set_transaction_to_sign(transaction_to_sign: *const u8, length: u64) -> u64);
define_syscall!(fn arch_get_bitcoin_tx(data: *mut u8, length: u64, txid: &[u8; 32]) -> u64);
define_syscall!(fn arch_get_runes_from_output(data: *mut u8, length: u64, txid: &[u8; 32], output_index: u32) -> u64);
define_syscall!(fn arch_get_network_xonly_pubkey(data: *mut u8) -> u64);
define_syscall!(fn arch_validate_utxo_ownership(utxo: *const UtxoMeta, owner: *const Pubkey) -> u64);
define_syscall!(fn arch_get_account_script_pubkey(script: *mut u8, pubkey: *const Pubkey) -> u64);
define_syscall!(fn arch_get_bitcoin_block_height() -> u64);
define_syscall!(fn arch_get_clock(clock: *mut Clock) -> u64);
define_syscall!(fn sol_secp256k1_recover( hash_addr: *const u8, recovery_id_val: u64, signature_addr: *const u8, result_addr: *mut u8) ->  u64);
define_syscall!(fn sol_keccak256(data: *const u8, length: u64, result: *mut u8) -> u64);

// logs
define_syscall!(fn sol_log_(message: *const u8, len: u64));
define_syscall!(fn sol_log_64_(arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64));
define_syscall!(fn sol_log_compute_units_());
define_syscall!(fn sol_log_pubkey(pubkey_addr: *const u8));
define_syscall!(fn sol_log_data(data: *const u8, data_len: u64));

define_syscall!(fn sol_memcmp_(data1: *const u8, data2: *const u8, len: u64, result: *mut i32) -> u64);
define_syscall!(fn sol_memcpy_(dest: *mut u8, src: *const u8, len: u64) -> u64);
define_syscall!(fn sol_memset_(dest: *mut u8, val: u8, len: u64) -> u64);
define_syscall!(fn sol_memmove_(dest: *mut u8, src: *const u8, len: u64) -> u64);

define_syscall!(fn sol_get_stack_height() -> u64);
