//! SPL Token account type with `SlabSchema` impl for use with `Account<T>`.
//!
//! Layout mirrors `pinocchio-token` — all fields are alignment-1 to support
//! zerocopy mapping from the account data buffer.

use {
    anchor_lang_v2::{
        accounts::{Account, SlabInit, SlabSchema},
        programs::Token,
        Constrain, Id,
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

impl Constrain<MintConstraint> for Account<TokenAccount> {
    #[inline(always)]
    fn constrain(&mut self, expected: &Address) -> Result<(), ProgramError> {
        // `address_eq` is the chunked 4×u64 compare; faster than the
        // default `PartialEq` derive on `[u8; 32]`. See lang-v2/src/lib.rs.
        if !anchor_lang_v2::address_eq(self.mint(), expected) {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

impl Constrain<AuthorityConstraint> for Account<TokenAccount> {
    #[inline(always)]
    fn constrain(&mut self, expected: &Address) -> Result<(), ProgramError> {
        if !anchor_lang_v2::address_eq(self.owner(), expected) {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

/// `token::TokenProgram = token_program` — check account is owned by given program.
impl Constrain<TokenProgramConstraint> for Account<TokenAccount> {
    #[inline(always)]
    fn constrain(&mut self, expected: &Address) -> Result<(), ProgramError> {
        if !AsRef::<AccountView>::as_ref(self).owned_by(expected) {
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
// Minimum viable surface — only `Transfer` and `TransferChecked` for now.
// Mirrors v1's `anchor_spl::token::cpi` path so users writing
// `anchor_spl_v2::token::cpi::transfer(ctx, amount)?` get the same
// ergonomics they had in v1.
//
// Each helper routes through `anchor_lang_v2::CpiContext::invoke`, which
// uses pinocchio's `invoke_signed_unchecked` — bypassing the borrow-state
// check that rejected the old `pinocchio_token::Transfer::invoke()` call
// against Slab-loaded `Account<TokenAccount>` accounts.
//
// TODO: add the rest of the SPL Token instruction surface here before
// this crate stabilizes:
//   - MintTo, MintToChecked
//   - Burn, BurnChecked
//   - Approve, ApproveChecked, Revoke
//   - SetAuthority
//   - CloseAccount
//   - SyncNative
//   - InitializeAccount[2/3], InitializeMint[2], InitializeMultisig[2]
//   - FreezeAccount, ThawAccount
// V1's `anchor_spl::token::cpi` covers all of these; porting is mostly
// mechanical (disc + data layout + accounts list), one function per
// instruction. We stop here to unblock the e2e tests — anything else
// wouldn't be exercised yet.
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
    }

    // SPL Token instruction discriminators — see the pinocchio-token crate
    // or the upstream spl-token source of truth.
    const DISC_TRANSFER: u8 = 3;
    const DISC_TRANSFER_CHECKED: u8 = 12;

    /// Invoke SPL Token `Transfer` with a fixed `amount`.
    ///
    /// The authority is the single source-account owner or a delegate.
    /// Multisig authorities are not supported by this helper — use a
    /// hand-built `CpiContext` if you need them.
    pub fn transfer<'a>(
        ctx: CpiContext<'a, accounts::Transfer<'a>>,
        amount: u64,
    ) -> Result<(), ProgramError> {
        // Layout: 1-byte discriminator + u64 amount LE = 9 bytes.
        let mut data = [0u8; 9];
        data[0] = DISC_TRANSFER;
        data[1..9].copy_from_slice(&amount.to_le_bytes());
        ctx.invoke(&data)
    }

    /// Invoke SPL Token `TransferChecked`. Extra on-chain safety over
    /// `transfer`: the SPL program verifies that `decimals` matches the
    /// mint's stored decimals and that the mint address matches the
    /// one threaded through the token accounts.
    pub fn transfer_checked<'a>(
        ctx: CpiContext<'a, accounts::TransferChecked<'a>>,
        amount: u64,
        decimals: u8,
    ) -> Result<(), ProgramError> {
        // Layout: 1-byte discriminator + u64 amount LE + u8 decimals = 10 bytes.
        let mut data = [0u8; 10];
        data[0] = DISC_TRANSFER_CHECKED;
        data[1..9].copy_from_slice(&amount.to_le_bytes());
        data[9] = decimals;
        ctx.invoke(&data)
    }
}
