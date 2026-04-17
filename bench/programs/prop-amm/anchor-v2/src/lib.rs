#![cfg_attr(target_os = "solana", feature(asm_experimental_arch))]

pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang_v2::prelude::*;

pub use instructions::*;

declare_id!("55555555555555555555555555555555555555555555");

/// ed25519 pubkey of `Keypair::new_from_array([7u8; 32])` — the tests
/// and bench builders reconstruct the same seed.
pub const UPDATE_AUTHORITY: Address = Address::new_from_array([
    234, 74, 108, 99, 226, 156, 82, 10, 190, 245, 80, 123, 19, 46, 197, 249, 149, 71, 118, 174,
    190, 190, 123, 146, 66, 30, 234, 105, 20, 70, 210, 44,
]);

#[program]
pub mod prop_amm_v2 {
    use super::*;

    /// Unreachable: the asm entrypoint handles `discrim=0` before this
    /// match arm runs. Declared so the macro emits `instruction::Update`
    /// + `accounts::Update` for clients and IDL.
    #[discrim = 0]
    #[allow(unused_variables)]
    pub fn update(ctx: &mut Context<Update>, new_price: u64) -> Result<()> {
        unsafe { core::hint::unreachable_unchecked() }
    }

    #[discrim = 1]
    pub fn initialize(ctx: &mut Context<Initialize>) -> Result<()> {
        ctx.accounts.oracle.authority = *ctx.accounts.payer.address();
        ctx.accounts.oracle.price = 0;
        Ok(())
    }

    #[discrim = 2]
    pub fn rotate_authority(
        ctx: &mut Context<RotateAuthority>,
        new_authority: [u8; 32],
    ) -> Result<()> {
        ctx.accounts.oracle.authority = Address::new_from_array(new_authority);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Update {
    #[account(mut)]
    pub oracle: Account<state::Oracle>,
    pub authority: Signer,
}

// The asm entrypoint handles `discrim=0` inline and tail-calls
// `__anchor_rust_dispatch` otherwise. `no-entrypoint` suppresses the
// macro's entrypoint + allocator + panic handler, so we reinstate them.
//
// Offsets are hardcoded against agave's aligned serialization, which
// lays account records out as:
//   [RuntimeAccount 88B | data | MAX_PERMITTED_DATA_INCREASE 10240B
//    | padding-to-8 | rent_epoch 8B]
// For an oracle with 48B data:
//   input + 0      num_accounts: u64
//   input + 8      account 0 (oracle) header
//   input + 10     oracle.is_writable
//   input + 88     oracle.data_len
//   input + 96     oracle.data[0..48]
//   input + 136    oracle.data[40..48]  (price slot)
//   input + 10392  account 1 (authority) header
//                  = 8 + 88 + 48 + 10240 + 8
//   input + 10393  authority.is_signer
//   input + 10400  authority.address[0..32]

#[cfg(target_os = "solana")]
anchor_lang_v2::pinocchio::default_allocator!();
#[cfg(target_os = "solana")]
anchor_lang_v2::pinocchio::default_panic_handler!();

// Linker symbol for the asm's non-zero-discrim branch to `call`. The macro
// emits `__anchor_dispatch` as `#[inline(always)]` with no symbol.
#[cfg(target_os = "solana")]
#[no_mangle]
pub unsafe extern "C" fn __anchor_rust_dispatch(
    input: *mut u8,
    ix_data_ptr: *const u8,
) -> u64 {
    __anchor_dispatch(input, ix_data_ptr)
}

#[cfg(target_os = "solana")]
core::arch::global_asm!(
    ".globl entrypoint",
    "entrypoint:",
    "    ldxb r3, [r2+0]",
    "    jne r3, 0, dispatch",
    "    ldxdw r3, [r1+0]",
    "    jlt r3, 2, err_few_accounts",
    "    ldxdw r3, [r1+88]",
    "    jne r3, 48, err_bad_data_len",
    "    ldxb r3, [r1+10]",
    "    jeq r3, 0, err_not_writable",
    "    ldxb r3, [r1+10393]",
    "    jeq r3, 0, err_not_signer",
    "    ldxdw r3, [r1+10400]",
    "    lddw r4, 0x0A529CE2636C4AEA",
    "    jne r3, r4, err_wrong_auth",
    "    ldxdw r3, [r1+10408]",
    "    lddw r4, 0xF9C52E137B50F5BE",
    "    jne r3, r4, err_wrong_auth",
    "    ldxdw r3, [r1+10416]",
    "    lddw r4, 0x927BBEBEAE764795",
    "    jne r3, r4, err_wrong_auth",
    "    ldxdw r3, [r1+10424]",
    "    lddw r4, 0x2CD2461469EA1E42",
    "    jne r3, r4, err_wrong_auth",
    // Unaligned 8B load: ix_data is aligned 1; sBPFv1+ allows this.
    "    ldxdw r3, [r2+1]",
    "    stxdw [r1+136], r3",
    "    mov64 r0, 0",
    "    exit",
    "err_few_accounts:",
    "    mov64 r0, 101",
    "    exit",
    "err_bad_data_len:",
    "    mov64 r0, 102",
    "    exit",
    "err_not_writable:",
    "    mov64 r0, 103",
    "    exit",
    "err_not_signer:",
    "    mov64 r0, 104",
    "    exit",
    "err_wrong_auth:",
    "    mov64 r0, 105",
    "    exit",
    "dispatch:",
    "    call __anchor_rust_dispatch",
    "    exit",
);
