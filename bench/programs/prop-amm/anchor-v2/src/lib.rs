#![cfg_attr(target_os = "solana", feature(asm_experimental_arch))]

pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang_v2::prelude::*;

pub use instructions::*;

declare_id!("55555555555555555555555555555555555555555555");

/// Hardcoded update authority. Corresponds to an ed25519 keypair
/// deterministically derived from `Keypair::new_from_array([7u8; 32])` —
/// the bench harness and the package's litesvm tests reconstruct this
/// same keypair to authorize price updates.
pub const UPDATE_AUTHORITY: Address = Address::new_from_array([
    234, 74, 108, 99, 226, 156, 82, 10, 190, 245, 80, 123, 19, 46, 197, 249, 149, 71, 118, 174,
    190, 190, 123, 146, 66, 30, 234, 105, 20, 70, 210, 44,
]);

#[program]
pub mod prop_amm_v2 {
    use super::*;

    /// Price update. The asm entrypoint short-circuits `discrim=0` directly
    /// into `__oracle_update`, so this match arm is unreachable on-chain —
    /// the body uses `unreachable_unchecked()` both to document the
    /// invariant and to avoid a `let _ = new_price;` dance. The declaration
    /// earns its keep by emitting `instruction::Update` and
    /// `accounts::Update` for clients + IDL.
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

/// Accounts struct for the `update` instruction. The on-chain path
/// hand-parses the account slots in `__oracle_update` and never goes
/// through `TryAccounts`; this declaration gives clients a
/// `to_account_metas()` that mirrors what the asm parser reads:
/// account 0 = oracle (mut), account 1 = authority (signer).
#[derive(Accounts)]
pub struct Update {
    #[account(mut)]
    pub oracle: Account<state::Oracle>,
    pub authority: Signer,
}

// =============================================================================
// Asm entrypoint + update helper
// =============================================================================
//
// The BPF loader jumps to `entrypoint` (SIMD-0321 2-arg: r1 = input region,
// r2 = ix_data_ptr). We load the 1-byte discriminator and branch:
//
//   * discrim == 0 (`update`) → call `__oracle_update`: hand-rolled walker
//     that verifies accounts, checks the hardcoded signer, and writes
//     `price` into the oracle account data. No allocator init, no anchor
//     dispatcher, no per-instruction log.
//
//   * discrim != 0 (`initialize`, `rotate_authority`) → call the macro-
//     generated `__anchor_dispatch` via a `#[no_mangle]` trampoline
//     (needed because `__anchor_dispatch` is emitted `#[inline(always)]`
//     with no linker symbol).
//
// `no-entrypoint` on the crate tells `#[program]` to skip its own
// entrypoint + allocator + panic handler; we reinstate them below.

#[cfg(target_os = "solana")]
anchor_lang_v2::pinocchio::default_allocator!();
#[cfg(target_os = "solana")]
anchor_lang_v2::pinocchio::default_panic_handler!();

#[cfg(target_os = "solana")]
#[no_mangle]
pub unsafe extern "C" fn __anchor_rust_dispatch(
    input: *mut u8,
    ix_data_ptr: *const u8,
) -> u64 {
    __anchor_dispatch(input, ix_data_ptr)
}

// Entrypoint, written entirely in hand-rolled sBPF. The loader hands us
// r1 = input region, r2 = ix_data_ptr. We branch on the discriminator
// byte: discrim == 0 runs the price update inline (no `call`, no
// function prologue, no Rust layer at all); any other discrim tail-calls
// into `__anchor_rust_dispatch` so `initialize` / `rotate_authority`
// still go through the anchor dispatcher.
//
// Account layout assumptions, hardcoded into the offsets below:
//   * `num_accounts: u64` at  input + 0
//   * Account 0 (oracle, 48B data) at       input + 8
//       RuntimeAccount header 88B, then 48B data
//       is_writable byte                    input + 10
//       data_len u64                        input + 88
//       data[0..48]                         input + 96
//       price slot (data + 40)              input + 136
//   * Account 1 (authority, signer) at      input + 10392
//       = input + 8 + 88 + 48 + 10240 (MAX_PERMITTED_DATA_INCREASE) + 8 (rent_epoch)
//       is_signer byte                      input + 10393
//       address 32B                         input + 10400..10432
//
// Error returns are `ProgramError::Custom(1xx)` — distinct small codes
// that fit a single `mov64 imm` (avoiding the 16-byte `lddw` sequence
// the agave builtin `ProgramError` variants would require).

#[cfg(target_os = "solana")]
core::arch::global_asm!(
    ".globl entrypoint",
    "entrypoint:",
    // r3 = *ix_data_ptr (discriminator byte)
    "    ldxb r3, [r2+0]",
    "    jne r3, 0, dispatch",
    // ---- Fast path (discrim == 0: update price) ----
    // num_accounts >= 2?
    "    ldxdw r3, [r1+0]",
    "    jlt r3, 2, err_few_accounts",
    // oracle.data_len == 48?
    "    ldxdw r3, [r1+88]",
    "    jne r3, 48, err_bad_data_len",
    // oracle.is_writable != 0?
    "    ldxb r3, [r1+10]",
    "    jeq r3, 0, err_not_writable",
    // authority.is_signer != 0?
    "    ldxb r3, [r1+10393]",
    "    jeq r3, 0, err_not_signer",
    // authority.address == UPDATE_AUTHORITY (32B split into 4 LE u64 chunks)
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
    // Copy 8-byte price: ix_data_ptr[1..9]  →  oracle_data[40..48]
    // (ix_data is aligned 1 so this is an unaligned 8B load; sBPFv1+
    // allows unaligned ldxdw/stxdw.)
    "    ldxdw r3, [r2+1]",
    "    stxdw [r1+136], r3",
    "    mov64 r0, 0",
    "    exit",
    // ---- Error exits (all ProgramError::Custom(N)) ----
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
    // ---- Normal anchor dispatcher (discrim != 0) ----
    "dispatch:",
    "    call __anchor_rust_dispatch",
    "    exit",
);
