//! Token-2022 extension data types and TLV parsing.
//!
//! Provides zero-copy access to extension data stored in Token-2022 accounts.
//! The TLV (Type-Length-Value) format is parsed inline — no dependency on
//! `spl-token-2022-interface`.
//!
//! # Usage
//!
//! ```ignore
//! use anchor_spl_v2::extensions::{self, TransferFeeConfig, MetadataPointer};
//!
//! // Read extension from an InterfaceAccount<Mint>
//! let fee_config: &TransferFeeConfig = extensions::get_mint_extension(&mint)?;
//! let metadata_ptr: &MetadataPointer = extensions::get_mint_extension(&mint)?;
//! ```

use {
    bytemuck::{self, Pod, Zeroable},
    pinocchio::account::AccountView,
    solana_address::Address,
    solana_program_error::ProgramError,
};

// ---------------------------------------------------------------------------
// TLV parser
// ---------------------------------------------------------------------------

/// TLV data starts at `Account::LEN + 1` (166) for ALL extensible accounts
/// (both Mint and TokenAccount). SPL pads mints to 165 bytes before the
/// AccountType marker so that mints and token accounts share the same layout
/// boundary. See spl-token-2022 extension/mod.rs for details.
const TLV_START: usize = 166;

/// Trait for extension types. Each extension has a u16 discriminant.
pub trait ExtensionType: Pod {
    const TYPE_DISCRIMINANT: u16;
}

/// Parse a fixed-size extension from a Token-2022 mint or token account.
///
/// Both mint and token account extensions share the same TLV start offset
/// (166 bytes from the beginning of account data).
pub fn get_mint_extension<T: ExtensionType>(
    account: &AccountView,
) -> Result<&T, ProgramError> {
    get_extension::<T>(account)
}

/// Parse a fixed-size extension from a Token-2022 token account.
pub fn get_token_account_extension<T: ExtensionType>(
    account: &AccountView,
) -> Result<&T, ProgramError> {
    get_extension::<T>(account)
}

fn get_extension<T: ExtensionType>(account: &AccountView) -> Result<&T, ProgramError> {
    let data = unsafe { account.borrow_unchecked() };
    if data.len() < TLV_START {
        return Err(ProgramError::InvalidAccountData);
    }
    get_extension_from_tlv::<T>(&data[TLV_START..])
}

/// Walk TLV entries and return a Pod reference to the matching extension.
fn get_extension_from_tlv<T: ExtensionType>(tlv_data: &[u8]) -> Result<&T, ProgramError> {
    let target = T::TYPE_DISCRIMINANT;
    let mut offset = 0;

    while offset + 4 <= tlv_data.len() {
        let ext_type = u16::from_le_bytes([tlv_data[offset], tlv_data[offset + 1]]);
        let length =
            u16::from_le_bytes([tlv_data[offset + 2], tlv_data[offset + 3]]) as usize;
        let value_end = offset + 4 + length;

        if ext_type == 0 {
            break;
        }
        if value_end > tlv_data.len() {
            return Err(ProgramError::InvalidAccountData);
        }

        if ext_type == target {
            let ext_size = core::mem::size_of::<T>();
            if length < ext_size {
                return Err(ProgramError::InvalidAccountData);
            }
            let value_start = offset + 4;
            return Ok(bytemuck::from_bytes(&tlv_data[value_start..value_start + ext_size]));
        }

        offset = value_end;
    }

    Err(ProgramError::InvalidAccountData)
}

// ---------------------------------------------------------------------------
// Extension struct definitions
//
// Layouts match spl-token-2022-interface exactly. All fields are alignment-1
// (Pod-safe). OptionalNonZeroPubkey is represented as Address — all-zeros
// means None.
// ---------------------------------------------------------------------------

/// Optional authority/address field. All-zeros represents "not set".
pub type OptionalAddress = Address;

/// Check if an optional address is set (non-zero).
#[inline(always)]
pub fn is_some_address(addr: &Address) -> bool {
    let ptr = addr.as_ref().as_ptr() as *const u64;
    unsafe { *ptr | *ptr.add(1) | *ptr.add(2) | *ptr.add(3) != 0 }
}

/// Return the address as an Option — None if all zeros.
#[inline(always)]
pub fn optional_address(addr: &Address) -> Option<&Address> {
    if is_some_address(addr) {
        Some(addr)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// TransferFeeConfig (ExtensionType = 1, mint extension)
// ---------------------------------------------------------------------------

/// Per-epoch transfer fee parameters.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TransferFee {
    /// First epoch where this fee takes effect.
    pub epoch: [u8; 8],
    /// Maximum fee assessed on transfers (token amount).
    pub maximum_fee: [u8; 8],
    /// Fee in basis points of the transfer amount (0.01% increments).
    pub transfer_fee_basis_points: [u8; 2],
}

impl TransferFee {
    pub fn epoch(&self) -> u64 {
        u64::from_le_bytes(self.epoch)
    }
    pub fn maximum_fee(&self) -> u64 {
        u64::from_le_bytes(self.maximum_fee)
    }
    pub fn basis_points(&self) -> u16 {
        u16::from_le_bytes(self.transfer_fee_basis_points)
    }
}

/// Transfer fee configuration on the mint.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TransferFeeConfig {
    pub transfer_fee_config_authority: OptionalAddress,
    pub withdraw_withheld_authority: OptionalAddress,
    pub withheld_amount: [u8; 8],
    pub older_transfer_fee: TransferFee,
    pub newer_transfer_fee: TransferFee,
}


impl TransferFeeConfig {
    pub fn withheld_amount(&self) -> u64 {
        u64::from_le_bytes(self.withheld_amount)
    }
}

impl ExtensionType for TransferFeeConfig {
    const TYPE_DISCRIMINANT: u16 = 1;
}

// ---------------------------------------------------------------------------
// TransferFeeAmount (ExtensionType = 2, account extension)
// ---------------------------------------------------------------------------

/// Withheld fee on a token account.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TransferFeeAmount {
    pub withheld_amount: [u8; 8],
}


impl TransferFeeAmount {
    pub fn withheld_amount(&self) -> u64 {
        u64::from_le_bytes(self.withheld_amount)
    }
}

impl ExtensionType for TransferFeeAmount {
    const TYPE_DISCRIMINANT: u16 = 2;
}

// ---------------------------------------------------------------------------
// MintCloseAuthority (ExtensionType = 3)
// ---------------------------------------------------------------------------

/// Authority that can close the mint.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct MintCloseAuthority {
    pub close_authority: OptionalAddress,
}


impl ExtensionType for MintCloseAuthority {
    const TYPE_DISCRIMINANT: u16 = 3;
}

// ---------------------------------------------------------------------------
// DefaultAccountState (ExtensionType = 6)
// ---------------------------------------------------------------------------

/// Default state for new token accounts created from this mint.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct DefaultAccountState {
    /// 1 = Initialized, 2 = Frozen
    pub state: u8,
}


impl ExtensionType for DefaultAccountState {
    const TYPE_DISCRIMINANT: u16 = 6;
}

// ---------------------------------------------------------------------------
// PermanentDelegate (ExtensionType = 12)
// ---------------------------------------------------------------------------

/// Permanent delegate for transferring or burning tokens from any account.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PermanentDelegate {
    pub delegate: OptionalAddress,
}


impl ExtensionType for PermanentDelegate {
    const TYPE_DISCRIMINANT: u16 = 12;
}

// ---------------------------------------------------------------------------
// TransferHook (ExtensionType = 14, mint extension)
// ---------------------------------------------------------------------------

/// Transfer hook configuration on the mint.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TransferHook {
    pub authority: OptionalAddress,
    pub program_id: OptionalAddress,
}


impl ExtensionType for TransferHook {
    const TYPE_DISCRIMINANT: u16 = 14;
}

// ---------------------------------------------------------------------------
// TransferHookAccount (ExtensionType = 15, account extension)
// ---------------------------------------------------------------------------

/// Transfer hook state on a token account.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TransferHookAccount {
    /// Whether the account is currently being transferred.
    pub transferring: u8,
}


impl ExtensionType for TransferHookAccount {
    const TYPE_DISCRIMINANT: u16 = 15;
}

// ---------------------------------------------------------------------------
// MetadataPointer (ExtensionType = 18, mint extension)
// ---------------------------------------------------------------------------

/// Points to the account holding token metadata.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct MetadataPointer {
    pub authority: OptionalAddress,
    pub metadata_address: OptionalAddress,
}


impl ExtensionType for MetadataPointer {
    const TYPE_DISCRIMINANT: u16 = 18;
}

// ---------------------------------------------------------------------------
// GroupPointer (ExtensionType = 20, mint extension)
// ---------------------------------------------------------------------------

/// Points to the account holding group configuration.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GroupPointer {
    pub authority: OptionalAddress,
    pub group_address: OptionalAddress,
}


impl ExtensionType for GroupPointer {
    const TYPE_DISCRIMINANT: u16 = 20;
}

// ---------------------------------------------------------------------------
// GroupMemberPointer (ExtensionType = 22, mint extension)
// ---------------------------------------------------------------------------

/// Points to the account holding group member configuration.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GroupMemberPointer {
    pub authority: OptionalAddress,
    pub member_address: OptionalAddress,
}


impl ExtensionType for GroupMemberPointer {
    const TYPE_DISCRIMINANT: u16 = 22;
}
