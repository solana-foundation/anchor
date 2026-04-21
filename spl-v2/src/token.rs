//! SPL Token account type with `SlabSchema` impl for use with `Account<T>`.
//!
//! Layout mirrors `pinocchio-token` — all fields are alignment-1 to support
//! zerocopy mapping from the account data buffer.

use {
    anchor_lang_v2::{
        accounts::{Account, SlabInit, SlabSchema},
        programs::Token,
        AccountConstraint, Id,
    },
    bytemuck::{Pod, Zeroable},
    pinocchio::account::AccountView,
    solana_address::Address,
    solana_program_error::ProgramError,
};

/// Create a Token-program-owned account, handling PDA signing if needed.
pub(crate) fn create_token_account(
    payer: &AccountView,
    account: &AccountView,
    space: usize,
    signer_seeds: Option<&[&[u8]]>,
) -> Result<(), ProgramError> {
    let token_program_id = Token::id();
    match signer_seeds {
        Some(seeds) => {
            anchor_lang_v2::create_account_signed(payer, account, space, &token_program_id, seeds)
        }
        None => anchor_lang_v2::create_account(payer, account, space, &token_program_id),
    }
}

/// SPL Token account data, zerocopy-mapped (165 bytes).
///
/// All fields are private — use the accessor methods to read data. Token
/// account state is modified only by the SPL Token program via CPI (Transfer,
/// MintTo, etc.); user programs cannot mutate these fields directly anyway
/// because the account is owned by the SPL Token program.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TokenAccount {
    mint: Address,
    owner: Address,
    amount: [u8; 8],
    delegate_flag: [u8; 4],
    delegate: Address,
    state: u8,
    is_native_flag: [u8; 4],
    native_amount: [u8; 8],
    delegated_amount: [u8; 8],
    close_authority_flag: [u8; 4],
    close_authority: Address,
}

// SAFETY: TokenAccount is repr(C) with all alignment-1 fields, no padding.
unsafe impl Pod for TokenAccount {}
unsafe impl Zeroable for TokenAccount {}


// TokenAccount is defined by the SPL Token program, not by the user's program
// — its layout is known to any SPL-aware client. Default `__IDL_TYPE = None`
// keeps it out of the user's IDL `types[]` array (matches v1's
// `impl_idl_build!` behavior for this type).
#[cfg(feature = "idl-build")]
impl anchor_lang_v2::IdlAccountType for TokenAccount {}

// On-chain size — SPL Token program requires 165 bytes. Used by
// `#[account(init, token::*)]` as the default when `space` is omitted.
impl anchor_lang_v2::Space for TokenAccount {
    const INIT_SPACE: usize = core::mem::size_of::<Self>();
}

impl SlabSchema for TokenAccount {
    // External types start at offset 0 — no Anchor discriminator.
    const DATA_OFFSET: usize = 0;

    #[inline(always)]
    fn validate(
        view: &AccountView,
        data: &[u8],
        _program_id: &Address,
    ) -> Result<(), ProgramError> {
        // TODO: Token2022 support — add a TokenAccount2022 type or a feature
        // gate that adds `|| view.owned_by(&Token2022::id())` here.
        if !view.owned_by(&Token::id()) {
            return Err(ProgramError::IllegalOwner);
        }
        // Exact size distinguishes TokenAccount (165) from Mint (82).
        if data.len() != core::mem::size_of::<Self>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

/// Init params for `#[account(init, token::mint = ..., token::authority = ...)]`.
#[derive(Default)]
pub struct TokenAccountInitParams<'a> {
    pub mint: Option<&'a AccountView>,
    pub authority: Option<&'a AccountView>,
}

impl SlabInit for TokenAccount {
    type Params<'a> = TokenAccountInitParams<'a>;

    #[cold]
    fn create_and_initialize<'a>(
        payer: &AccountView,
        account: &AccountView,
        _space: usize,
        _program_id: &Address,
        params: &Self::Params<'a>,
        signer_seeds: Option<&[&[u8]]>,
    ) -> Result<(), ProgramError> {
        let mint = params.mint.ok_or(ProgramError::InvalidArgument)?;
        let authority = params.authority.ok_or(ProgramError::InvalidArgument)?;

        create_token_account(payer, account, core::mem::size_of::<Self>(), signer_seeds)?;

        pinocchio_token::instructions::InitializeAccount3 {
            account,
            mint,
            owner: authority.address(),
        }
        .invoke()
    }
}

impl TokenAccount {
    /// The mint associated with this token account.
    pub fn mint(&self) -> &Address {
        &self.mint
    }

    /// The owner of this token account.
    pub fn owner(&self) -> &Address {
        &self.owner
    }

    /// The token balance.
    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    /// The amount currently delegated.
    pub fn delegated_amount(&self) -> u64 {
        u64::from_le_bytes(self.delegated_amount)
    }

    /// Whether a delegate is currently approved.
    pub fn has_delegate(&self) -> bool {
        self.delegate_flag[0] == 1
    }

    /// The approved delegate, if any.
    pub fn delegate(&self) -> Option<&Address> {
        if self.has_delegate() {
            Some(&self.delegate)
        } else {
            None
        }
    }

    /// Account state (0 = Uninitialized, 1 = Initialized, 2 = Frozen).
    pub fn state(&self) -> u8 {
        self.state
    }

    /// Whether this is a wrapped SOL account.
    pub fn is_native(&self) -> bool {
        self.is_native_flag[0] == 1
    }

    /// The rent-exempt reserve for native SOL accounts, if this is a native
    /// token account.
    pub fn native_amount(&self) -> Option<u64> {
        if self.is_native() {
            Some(u64::from_le_bytes(self.native_amount))
        } else {
            None
        }
    }

    /// Whether a close authority is set.
    pub fn has_close_authority(&self) -> bool {
        self.close_authority_flag[0] == 1
    }

    /// The close authority, if any.
    pub fn close_authority(&self) -> Option<&Address> {
        if self.has_close_authority() {
            Some(&self.close_authority)
        } else {
            None
        }
    }

    /// Whether the account has been initialized (state != 0).
    pub fn is_initialized(&self) -> bool {
        self.state != 0
    }

    /// Whether the account is frozen (state == 2).
    pub fn is_frozen(&self) -> bool {
        self.state == 2
    }
}

// ---------------------------------------------------------------------------
// Constraint markers for `#[account(token::*)]`
// ---------------------------------------------------------------------------

pub struct MintConstraint;
pub struct AuthorityConstraint;
pub struct TokenProgramConstraint;

impl AccountConstraint<Account<TokenAccount>> for MintConstraint {
    type Value = Address;
    #[inline(always)]
    fn check(account: &Account<TokenAccount>, expected: &Address) -> Result<(), ProgramError> {
        if !anchor_lang_v2::address_eq(account.mint(), expected) {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

impl AccountConstraint<Account<TokenAccount>> for AuthorityConstraint {
    type Value = Address;
    #[inline(always)]
    fn check(account: &Account<TokenAccount>, expected: &Address) -> Result<(), ProgramError> {
        if !anchor_lang_v2::address_eq(account.owner(), expected) {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

/// `token::TokenProgram = token_program` — check account is owned by given program.
impl AccountConstraint<Account<TokenAccount>> for TokenProgramConstraint {
    type Value = Address;
    #[inline(always)]
    fn check(account: &Account<TokenAccount>, expected: &Address) -> Result<(), ProgramError> {
        if !AsRef::<AccountView>::as_ref(account).owned_by(expected) {
            Err(ProgramError::IllegalOwner)
        } else {
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// CPI helpers for SPL Token program invocations (`anchor_spl_v2::token::cpi`)
// ---------------------------------------------------------------------------
//
// Mirrors v1's `anchor_spl::token::cpi` path. Each helper routes through
// `CpiContext::invoke` (pinocchio `invoke_signed_unchecked`), bypassing
// the borrow-state check that rejects direct pinocchio-token invoke on
// Slab-loaded accounts.
//
pub mod cpi {
    extern crate alloc;
    use {
        alloc::{vec, vec::Vec},
        anchor_lang_v2::{CpiContext, CpiHandle, ToCpiAccounts},
        pinocchio::instruction::InstructionAccount,
        solana_program_error::ProgramError,
    };

    /// Accounts structs consumed by each CPI helper. Each field is a
    /// `CpiHandle<'a>` obtained from `AnchorAccount::cpi_handle{,_mut}`.
    pub mod accounts {
        use super::*;

        /// `spl_token::instruction::transfer` — accounts list:
        ///   0. `[writable]` from
        ///   1. `[writable]` to
        ///   2. `[signer]` authority (owner/delegate)
        pub struct Transfer<'a> {
            pub from: CpiHandle<'a>,
            pub to: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for Transfer<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.from.address()),
                    InstructionAccount::writable(self.to.address()),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }

            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.from, self.to, self.authority]
            }
        }

        /// `spl_token::instruction::transfer_checked` — adds the mint and
        /// verifies the declared decimals match on-chain.
        ///   0. `[writable]` from
        ///   1. `[]` mint
        ///   2. `[writable]` to
        ///   3. `[signer]` authority
        pub struct TransferChecked<'a> {
            pub from: CpiHandle<'a>,
            pub mint: CpiHandle<'a>,
            pub to: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for TransferChecked<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.from.address()),
                    InstructionAccount::new(self.mint.address(), false, false),
                    InstructionAccount::writable(self.to.address()),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }

            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.from, self.mint, self.to, self.authority]
            }
        }

        pub struct MintTo<'a> {
            pub mint: CpiHandle<'a>,
            pub to: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for MintTo<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.mint.address()),
                    InstructionAccount::writable(self.to.address()),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }
            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.mint, self.to, self.authority]
            }
        }

        pub struct MintToChecked<'a> {
            pub mint: CpiHandle<'a>,
            pub to: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for MintToChecked<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.mint.address()),
                    InstructionAccount::writable(self.to.address()),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }
            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.mint, self.to, self.authority]
            }
        }

        pub struct Burn<'a> {
            pub account: CpiHandle<'a>,
            pub mint: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for Burn<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.account.address()),
                    InstructionAccount::writable(self.mint.address()),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }
            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.account, self.mint, self.authority]
            }
        }

        pub struct BurnChecked<'a> {
            pub account: CpiHandle<'a>,
            pub mint: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for BurnChecked<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.account.address()),
                    InstructionAccount::writable(self.mint.address()),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }
            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.account, self.mint, self.authority]
            }
        }

        pub struct Approve<'a> {
            pub source: CpiHandle<'a>,
            pub delegate: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for Approve<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.source.address()),
                    InstructionAccount::new(self.delegate.address(), false, false),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }
            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.source, self.delegate, self.authority]
            }
        }

        pub struct ApproveChecked<'a> {
            pub source: CpiHandle<'a>,
            pub mint: CpiHandle<'a>,
            pub delegate: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for ApproveChecked<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.source.address()),
                    InstructionAccount::new(self.mint.address(), false, false),
                    InstructionAccount::new(self.delegate.address(), false, false),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }
            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.source, self.mint, self.delegate, self.authority]
            }
        }

        pub struct Revoke<'a> {
            pub source: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for Revoke<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.source.address()),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }
            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.source, self.authority]
            }
        }

        pub struct SetAuthority<'a> {
            pub account: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for SetAuthority<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.account.address()),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }
            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.account, self.authority]
            }
        }

        pub struct CloseAccount<'a> {
            pub account: CpiHandle<'a>,
            pub destination: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for CloseAccount<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.account.address()),
                    InstructionAccount::writable(self.destination.address()),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }
            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.account, self.destination, self.authority]
            }
        }

        pub struct FreezeAccount<'a> {
            pub account: CpiHandle<'a>,
            pub mint: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for FreezeAccount<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.account.address()),
                    InstructionAccount::new(self.mint.address(), false, false),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }
            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.account, self.mint, self.authority]
            }
        }

        pub struct ThawAccount<'a> {
            pub account: CpiHandle<'a>,
            pub mint: CpiHandle<'a>,
            pub authority: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for ThawAccount<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![
                    InstructionAccount::writable(self.account.address()),
                    InstructionAccount::new(self.mint.address(), false, false),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ]
            }
            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.account, self.mint, self.authority]
            }
        }

        pub struct SyncNative<'a> {
            pub account: CpiHandle<'a>,
        }

        impl<'a> ToCpiAccounts<'a> for SyncNative<'a> {
            fn to_instruction_accounts(&self) -> Vec<InstructionAccount<'a>> {
                vec![InstructionAccount::writable(self.account.address())]
            }
            fn to_cpi_handles(&self) -> Vec<CpiHandle<'a>> {
                vec![self.account]
            }
        }
    }

    const DISC_TRANSFER: u8 = 3;
    const DISC_APPROVE: u8 = 4;
    const DISC_REVOKE: u8 = 5;
    const DISC_SET_AUTHORITY: u8 = 6;
    const DISC_MINT_TO: u8 = 7;
    const DISC_BURN: u8 = 8;
    const DISC_CLOSE_ACCOUNT: u8 = 9;
    const DISC_FREEZE_ACCOUNT: u8 = 10;
    const DISC_THAW_ACCOUNT: u8 = 11;
    const DISC_TRANSFER_CHECKED: u8 = 12;
    const DISC_APPROVE_CHECKED: u8 = 13;
    const DISC_MINT_TO_CHECKED: u8 = 14;
    const DISC_BURN_CHECKED: u8 = 15;
    const DISC_SYNC_NATIVE: u8 = 17;

    // Encode an SPL Token instruction with layout `[disc u8][amount u64 LE]`.
    //
    // Extracted as a pure helper so the Kani harness below can verify the
    // real encoder byte-for-byte. Without this extraction, harnesses would
    // have to rebuild the layout inline, leaving the production encoders
    // (`transfer`, `mint_to`, `burn`, `approve`) unverified — they bury the
    // encoding inside functions whose ultimate `ctx.invoke` is opaque to
    // CBMC.
    #[inline]
    fn encode_amount_ix(disc: u8, amount: u64) -> [u8; 9] {
        let mut data = [0u8; 9];
        data[0] = disc;
        data[1..9].copy_from_slice(&amount.to_le_bytes());
        data
    }

    // `*Checked` variants add a trailing `decimals` byte — same Kani rationale.
    #[inline]
    fn encode_amount_decimals_ix(disc: u8, amount: u64, decimals: u8) -> [u8; 10] {
        let mut data = [0u8; 10];
        data[0] = disc;
        data[1..9].copy_from_slice(&amount.to_le_bytes());
        data[9] = decimals;
        data
    }

    pub fn transfer<'a>(
        ctx: CpiContext<'a, accounts::Transfer<'a>>,
        amount: u64,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&encode_amount_ix(DISC_TRANSFER, amount))
    }

    pub fn transfer_checked<'a>(
        ctx: CpiContext<'a, accounts::TransferChecked<'a>>,
        amount: u64,
        decimals: u8,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&encode_amount_decimals_ix(DISC_TRANSFER_CHECKED, amount, decimals))
    }

    pub fn mint_to<'a>(
        ctx: CpiContext<'a, accounts::MintTo<'a>>,
        amount: u64,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&encode_amount_ix(DISC_MINT_TO, amount))
    }

    pub fn mint_to_checked<'a>(
        ctx: CpiContext<'a, accounts::MintToChecked<'a>>,
        amount: u64,
        decimals: u8,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&encode_amount_decimals_ix(DISC_MINT_TO_CHECKED, amount, decimals))
    }

    pub fn burn<'a>(
        ctx: CpiContext<'a, accounts::Burn<'a>>,
        amount: u64,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&encode_amount_ix(DISC_BURN, amount))
    }

    pub fn burn_checked<'a>(
        ctx: CpiContext<'a, accounts::BurnChecked<'a>>,
        amount: u64,
        decimals: u8,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&encode_amount_decimals_ix(DISC_BURN_CHECKED, amount, decimals))
    }

    pub fn approve<'a>(
        ctx: CpiContext<'a, accounts::Approve<'a>>,
        amount: u64,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&encode_amount_ix(DISC_APPROVE, amount))
    }

    pub fn approve_checked<'a>(
        ctx: CpiContext<'a, accounts::ApproveChecked<'a>>,
        amount: u64,
        decimals: u8,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&encode_amount_decimals_ix(DISC_APPROVE_CHECKED, amount, decimals))
    }

    pub fn revoke<'a>(
        ctx: CpiContext<'a, accounts::Revoke<'a>>,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&[DISC_REVOKE])
    }

    /// SPL Token `SetAuthority`.
    ///
    /// `authority_type`: 0 = MintTokens, 1 = FreezeAccount, 2 = AccountOwner,
    /// 3 = CloseAccount. See `spl_token::instruction::AuthorityType`.
    ///
    /// `new_authority`: `Some(address)` to set, `None` to revoke.
    pub fn set_authority<'a>(
        ctx: CpiContext<'a, accounts::SetAuthority<'a>>,
        authority_type: u8,
        new_authority: Option<&anchor_lang_v2::Address>,
    ) -> Result<(), ProgramError> {
        // Layout: disc(1) + authority_type(1) + option_tag(1) + [address(32)] = 3 or 35.
        let mut data = [0u8; 35];
        data[0] = DISC_SET_AUTHORITY;
        data[1] = authority_type;
        match new_authority {
            Some(addr) => {
                data[2] = 1;
                data[3..35].copy_from_slice(addr.as_ref());
                ctx.invoke(&data[..35])
            }
            None => {
                data[2] = 0;
                ctx.invoke(&data[..3])
            }
        }
    }

    pub fn close_account<'a>(
        ctx: CpiContext<'a, accounts::CloseAccount<'a>>,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&[DISC_CLOSE_ACCOUNT])
    }

    pub fn freeze_account<'a>(
        ctx: CpiContext<'a, accounts::FreezeAccount<'a>>,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&[DISC_FREEZE_ACCOUNT])
    }

    pub fn thaw_account<'a>(
        ctx: CpiContext<'a, accounts::ThawAccount<'a>>,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&[DISC_THAW_ACCOUNT])
    }

    pub fn sync_native<'a>(
        ctx: CpiContext<'a, accounts::SyncNative<'a>>,
    ) -> Result<(), ProgramError> {
        ctx.invoke(&[DISC_SYNC_NATIVE])
    }
}

