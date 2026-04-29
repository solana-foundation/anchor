//! Test program exercising `anchor-spl-v2`'s Mint/TokenAccount surface.
//!
//! Each handler targets a specific area of the SPL module — init codegen,
//! CPI helpers, accessor methods, namespaced constraints — so the
//! integration tests in `tests/spl.rs` can trip each path from a known
//! state and coverage attributes the execution back to the right file.

use {
    anchor_lang_v2::prelude::*,
    anchor_spl_v2::{
        associated_token::get_associated_token_address,
        extensions::{
            self, MetadataPointer, MintCloseAuthority, PermanentDelegate, TransferFeeAmount,
            TransferFeeConfig, TransferHook, TransferHookAccount,
        },
        mint::{self, Mint},
        token::{self, cpi as token_cpi, TokenAccount},
        token_interface::InterfaceAccount,
    },
};

declare_id!("SpL1111111111111111111111111111111111111111");

#[program]
pub mod spl_test {
    use super::*;

    /// Create a new Mint account. Hits `mint::SlabInit::create_and_initialize`
    /// → `pinocchio_token::InitializeMint2`.
    #[discrim = 0]
    pub fn init_mint(_ctx: &mut Context<InitMint>) -> Result<()> {
        Ok(())
    }

    /// Create a new TokenAccount. Hits `token::SlabInit::create_and_initialize`
    /// → `pinocchio_token::InitializeAccount3`.
    #[discrim = 1]
    pub fn init_token_account(_ctx: &mut Context<InitTokenAccount>) -> Result<()> {
        Ok(())
    }

    /// Mint `amount` tokens into `to`. Hits `token_cpi::mint_to`.
    #[discrim = 2]
    pub fn do_mint_to(ctx: &mut Context<DoMintTo>, amount: u64) -> Result<()> {
        let accs = token_cpi::accounts::MintTo {
            mint: ctx.accounts.mint.cpi_handle_mut(),
            to: ctx.accounts.to.cpi_handle_mut(),
            authority: ctx.accounts.authority.cpi_handle(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.address(), accs);
        token_cpi::mint_to(cpi_ctx, amount)
    }

    /// Transfer `amount` tokens from `from` to `to`. Hits `token_cpi::transfer`.
    #[discrim = 3]
    pub fn do_transfer(ctx: &mut Context<DoTransfer>, amount: u64) -> Result<()> {
        let accs = token_cpi::accounts::Transfer {
            from: ctx.accounts.from.cpi_handle_mut(),
            to: ctx.accounts.to.cpi_handle_mut(),
            authority: ctx.accounts.authority.cpi_handle(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.address(), accs);
        token_cpi::transfer(cpi_ctx, amount)
    }

    /// TransferChecked (also verifies decimals match mint). Hits
    /// `token_cpi::transfer_checked`.
    #[discrim = 4]
    pub fn do_transfer_checked(
        ctx: &mut Context<DoTransferChecked>,
        amount: u64,
        decimals: u8,
    ) -> Result<()> {
        let accs = token_cpi::accounts::TransferChecked {
            from: ctx.accounts.from.cpi_handle_mut(),
            mint: ctx.accounts.mint.cpi_handle(),
            to: ctx.accounts.to.cpi_handle_mut(),
            authority: ctx.accounts.authority.cpi_handle(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.address(), accs);
        token_cpi::transfer_checked(cpi_ctx, amount, decimals)
    }

    /// Burn `amount` tokens from `account`. Hits `token_cpi::burn`.
    #[discrim = 5]
    pub fn do_burn(ctx: &mut Context<DoBurn>, amount: u64) -> Result<()> {
        let accs = token_cpi::accounts::Burn {
            account: ctx.accounts.account.cpi_handle_mut(),
            mint: ctx.accounts.mint.cpi_handle_mut(),
            authority: ctx.accounts.authority.cpi_handle(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.address(), accs);
        token_cpi::burn(cpi_ctx, amount)
    }

    /// Approve `delegate` to spend `amount` from `source`. Hits
    /// `token_cpi::approve`.
    #[discrim = 6]
    pub fn do_approve(ctx: &mut Context<DoApprove>, amount: u64) -> Result<()> {
        let accs = token_cpi::accounts::Approve {
            source: ctx.accounts.source.cpi_handle_mut(),
            delegate: ctx.accounts.delegate.cpi_handle(),
            authority: ctx.accounts.authority.cpi_handle(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.address(), accs);
        token_cpi::approve(cpi_ctx, amount)
    }

    /// Revoke delegation. Hits `token_cpi::revoke`.
    #[discrim = 7]
    pub fn do_revoke(ctx: &mut Context<DoRevoke>) -> Result<()> {
        let accs = token_cpi::accounts::Revoke {
            source: ctx.accounts.source.cpi_handle_mut(),
            authority: ctx.accounts.authority.cpi_handle(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.address(), accs);
        token_cpi::revoke(cpi_ctx)
    }

    /// Close `account`, reclaiming lamports to `destination`. Hits
    /// `token_cpi::close_account`.
    #[discrim = 8]
    pub fn do_close_account(ctx: &mut Context<DoCloseAccount>) -> Result<()> {
        let accs = token_cpi::accounts::CloseAccount {
            account: ctx.accounts.account.cpi_handle_mut(),
            destination: ctx.accounts.destination.cpi_handle_mut(),
            authority: ctx.accounts.authority.cpi_handle(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.address(), accs);
        token_cpi::close_account(cpi_ctx)
    }

    /// Reads every `Mint` accessor — supply, decimals, authority flags,
    /// freeze flags. Logs nothing (logging costs CUs); the assertion is that
    /// the call succeeds and the traces cover the accessor methods.
    #[discrim = 9]
    pub fn read_mint(ctx: &mut Context<ReadMint>) -> Result<()> {
        let m = &*ctx.accounts.mint;
        let _ = m.supply();
        let _ = m.decimals();
        let _ = m.has_mint_authority();
        let _ = m.mint_authority();
        let _ = m.is_initialized();
        let _ = m.has_freeze_authority();
        let _ = m.freeze_authority();
        Ok(())
    }

    /// Reads every `TokenAccount` accessor — amount, delegate flags, state,
    /// native/close flags. See `read_mint` for rationale.
    #[discrim = 10]
    pub fn read_token_account(ctx: &mut Context<ReadTokenAccount>) -> Result<()> {
        let ta = &*ctx.accounts.token_account;
        let _ = ta.mint();
        let _ = ta.owner();
        let _ = ta.amount();
        let _ = ta.delegated_amount();
        let _ = ta.has_delegate();
        let _ = ta.delegate();
        let _ = ta.state();
        let _ = ta.is_native();
        let _ = ta.native_amount();
        let _ = ta.has_close_authority();
        let _ = ta.close_authority();
        let _ = ta.is_initialized();
        let _ = ta.is_frozen();
        Ok(())
    }

    /// `mint::decimals = 6` constraint. Tests pass a mint with matching
    /// decimals; mismatch path asserts the `InvalidAccountData` response.
    #[discrim = 11]
    pub fn check_mint_decimals(_ctx: &mut Context<CheckMintDecimals>) -> Result<()> {
        Ok(())
    }

    /// `mint::authority = expected` constraint.
    #[discrim = 12]
    pub fn check_mint_authority(_ctx: &mut Context<CheckMintAuthority>) -> Result<()> {
        Ok(())
    }

    /// `token::mint = mint` constraint.
    #[discrim = 13]
    pub fn check_token_mint(_ctx: &mut Context<CheckTokenMint>) -> Result<()> {
        Ok(())
    }

    /// `token::authority = expected` constraint.
    #[discrim = 14]
    pub fn check_token_authority(_ctx: &mut Context<CheckTokenAuthority>) -> Result<()> {
        Ok(())
    }

    /// Verifies that `vault` is the canonical ATA for `(authority, mint)`.
    /// Exercises `get_associated_token_address`.
    #[discrim = 15]
    pub fn check_ata(ctx: &mut Context<CheckAta>) -> Result<()> {
        let expected = get_associated_token_address(
            ctx.accounts.authority.account().address(),
            ctx.accounts.mint.account().address(),
            &anchor_lang_v2::programs::Token::id(),
        );
        if *ctx.accounts.vault.account().address() != expected {
            return Err(ProgramError::InvalidAccountData.into());
        }
        Ok(())
    }

    // ---- InterfaceAccount read path ---------------------------------------

    /// Load an `InterfaceAccount<Mint>` and touch accessors. Succeeds when the
    /// underlying account is owned by either Token or Token-2022.
    #[discrim = 16]
    pub fn read_interface_mint(ctx: &mut Context<ReadInterfaceMint>) -> Result<()> {
        let m = &*ctx.accounts.mint;
        let _ = m.supply();
        let _ = m.decimals();
        let _ = m.mint_authority();
        let _ = m.freeze_authority();
        let _ = m.is_initialized();
        Ok(())
    }

    /// Load an `InterfaceAccount<TokenAccount>` and touch accessors.
    #[discrim = 17]
    pub fn read_interface_token_account(
        ctx: &mut Context<ReadInterfaceTokenAccount>,
    ) -> Result<()> {
        let ta = &*ctx.accounts.token_account;
        let _ = ta.mint();
        let _ = ta.owner();
        let _ = ta.amount();
        Ok(())
    }

    // ---- InterfaceAccount init path ---------------------------------------

    /// Create a new Mint through the `InterfaceAccount<Mint>` init path with
    /// the legacy Token program (InitializeMint2 is hardcoded to legacy).
    #[discrim = 18]
    pub fn init_interface_mint(_ctx: &mut Context<InitInterfaceMint>) -> Result<()> {
        Ok(())
    }

    /// Create a new TokenAccount through the `InterfaceAccount<TokenAccount>`
    /// init path with the legacy Token program.
    #[discrim = 19]
    pub fn init_interface_token_account(
        _ctx: &mut Context<InitInterfaceTokenAccount>,
    ) -> Result<()> {
        Ok(())
    }

    // ---- Namespaced constraints on InterfaceAccount -----------------------

    /// `token::mint = mint` on `InterfaceAccount<TokenAccount>`.
    #[discrim = 20]
    pub fn check_interface_token_mint(
        _ctx: &mut Context<CheckInterfaceTokenMint>,
    ) -> Result<()> {
        Ok(())
    }

    /// `token::authority = expected` on `InterfaceAccount<TokenAccount>`.
    #[discrim = 21]
    pub fn check_interface_token_authority(
        _ctx: &mut Context<CheckInterfaceTokenAuthority>,
    ) -> Result<()> {
        Ok(())
    }

    /// `token::token_program = token_program` on `InterfaceAccount<TokenAccount>`.
    #[discrim = 22]
    pub fn check_interface_token_program(
        _ctx: &mut Context<CheckInterfaceTokenProgram>,
    ) -> Result<()> {
        Ok(())
    }

    /// `mint::authority = expected` on `InterfaceAccount<Mint>`.
    #[discrim = 23]
    pub fn check_interface_mint_authority(
        _ctx: &mut Context<CheckInterfaceMintAuthority>,
    ) -> Result<()> {
        Ok(())
    }

    /// `mint::freeze_authority = expected` on `InterfaceAccount<Mint>`.
    #[discrim = 24]
    pub fn check_interface_mint_freeze_authority(
        _ctx: &mut Context<CheckInterfaceMintFreezeAuthority>,
    ) -> Result<()> {
        Ok(())
    }

    /// `mint::decimals = 6` on `InterfaceAccount<Mint>`.
    #[discrim = 25]
    pub fn check_interface_mint_decimals(
        _ctx: &mut Context<CheckInterfaceMintDecimals>,
    ) -> Result<()> {
        Ok(())
    }

    /// `mint::token_program = token_program` on `InterfaceAccount<Mint>`.
    #[discrim = 26]
    pub fn check_interface_mint_token_program(
        _ctx: &mut Context<CheckInterfaceMintTokenProgram>,
    ) -> Result<()> {
        Ok(())
    }

    // ---- Token-2022 extension parsing -------------------------------------

    /// Parse `TransferFeeConfig` from a Token-2022 mint and assert the
    /// newer transfer fee's basis-points value matches `expected_bps`.
    #[discrim = 27]
    pub fn read_transfer_fee_config(
        ctx: &mut Context<ReadTransferFeeConfig>,
        expected_bps: u16,
    ) -> Result<()> {
        let ext: &TransferFeeConfig =
            extensions::get_mint_extension(ctx.accounts.mint.account())?;
        if ext.newer_transfer_fee.basis_points() != expected_bps {
            return Err(ProgramError::InvalidAccountData.into());
        }
        Ok(())
    }

    /// Parse `MetadataPointer` and assert both the `authority` and
    /// `metadata_address` fields match the passed pubkeys.
    #[discrim = 28]
    pub fn read_metadata_pointer(
        ctx: &mut Context<ReadMetadataPointer>,
        expected_authority: Address,
        expected_metadata: Address,
    ) -> Result<()> {
        let ext: &MetadataPointer =
            extensions::get_mint_extension(ctx.accounts.mint.account())?;
        if ext.authority != expected_authority || ext.metadata_address != expected_metadata {
            return Err(ProgramError::InvalidAccountData.into());
        }
        Ok(())
    }

    /// Parse `TransferHook` and assert the `program_id` matches.
    #[discrim = 29]
    pub fn read_transfer_hook(
        ctx: &mut Context<ReadTransferHook>,
        expected_program_id: Address,
    ) -> Result<()> {
        let ext: &TransferHook =
            extensions::get_mint_extension(ctx.accounts.mint.account())?;
        if ext.program_id != expected_program_id {
            return Err(ProgramError::InvalidAccountData.into());
        }
        Ok(())
    }

    /// Parse `MintCloseAuthority`, exercising `optional_address` on the
    /// resulting field — returns an error when the close authority is unset.
    #[discrim = 30]
    pub fn read_mint_close_authority(
        ctx: &mut Context<ReadMintCloseAuthority>,
        expected_authority: Address,
    ) -> Result<()> {
        let ext: &MintCloseAuthority =
            extensions::get_mint_extension(ctx.accounts.mint.account())?;
        match extensions::optional_address(&ext.close_authority) {
            Some(addr) if *addr == expected_authority => Ok(()),
            _ => Err(ProgramError::InvalidAccountData.into()),
        }
    }

    /// Parse `PermanentDelegate` and compare via `optional_address`.
    #[discrim = 31]
    pub fn read_permanent_delegate(
        ctx: &mut Context<ReadPermanentDelegate>,
        expected_delegate: Address,
    ) -> Result<()> {
        let ext: &PermanentDelegate =
            extensions::get_mint_extension(ctx.accounts.mint.account())?;
        match extensions::optional_address(&ext.delegate) {
            Some(addr) if *addr == expected_delegate => Ok(()),
            _ => Err(ProgramError::InvalidAccountData.into()),
        }
    }

    /// Parse `TransferFeeAmount` from a Token-2022 token account.
    #[discrim = 32]
    pub fn read_transfer_fee_amount(
        ctx: &mut Context<ReadTransferFeeAmount>,
        expected_withheld: u64,
    ) -> Result<()> {
        let ext: &TransferFeeAmount =
            extensions::get_token_account_extension(ctx.accounts.token_account.account())?;
        if ext.withheld_amount() != expected_withheld {
            return Err(ProgramError::InvalidAccountData.into());
        }
        Ok(())
    }

    /// Parse `TransferHookAccount` from a Token-2022 token account.
    #[discrim = 33]
    pub fn read_transfer_hook_account(
        ctx: &mut Context<ReadTransferHookAccount>,
        expected_transferring: u8,
    ) -> Result<()> {
        let ext: &TransferHookAccount =
            extensions::get_token_account_extension(ctx.accounts.token_account.account())?;
        if ext.transferring != expected_transferring {
            return Err(ProgramError::InvalidAccountData.into());
        }
        Ok(())
    }
}

// -- Accounts structs --------------------------------------------------------
//
// Sibling field refs used in namespaced constraints (e.g. `mint::authority
// = authority`) must appear above the field that references them —
// `try_accounts` loads fields in declaration order and codegen emits the
// Constrain call after all earlier fields have been pulled.

#[derive(Accounts)]
pub struct InitMint {
    #[account(mut)]
    pub payer: Signer,
    pub authority: UncheckedAccount,
    #[account(
        init,
        payer = payer,
        mint::decimals = 6,
        mint::authority = authority,
    )]
    pub mint: Account<Mint>,
    pub token_program: Program<Token>,
    pub system_program: Program<System>,
}

#[derive(Accounts)]
pub struct InitTokenAccount {
    #[account(mut)]
    pub payer: Signer,
    pub mint: Account<Mint>,
    pub authority: UncheckedAccount,
    #[account(
        init,
        payer = payer,
        token::mint = mint,
        token::authority = authority,
    )]
    pub token_account: Account<TokenAccount>,
    pub token_program: Program<Token>,
    pub system_program: Program<System>,
}

#[derive(Accounts)]
pub struct DoMintTo {
    #[account(mut)]
    pub mint: Account<Mint>,
    #[account(mut)]
    pub to: Account<TokenAccount>,
    pub authority: Signer,
    pub token_program: Program<Token>,
}

#[derive(Accounts)]
pub struct DoTransfer {
    #[account(mut)]
    pub from: Account<TokenAccount>,
    #[account(mut)]
    pub to: Account<TokenAccount>,
    pub authority: Signer,
    pub token_program: Program<Token>,
}

#[derive(Accounts)]
pub struct DoTransferChecked {
    #[account(mut)]
    pub from: Account<TokenAccount>,
    pub mint: Account<Mint>,
    #[account(mut)]
    pub to: Account<TokenAccount>,
    pub authority: Signer,
    pub token_program: Program<Token>,
}

#[derive(Accounts)]
pub struct DoBurn {
    #[account(mut)]
    pub account: Account<TokenAccount>,
    #[account(mut)]
    pub mint: Account<Mint>,
    pub authority: Signer,
    pub token_program: Program<Token>,
}

#[derive(Accounts)]
pub struct DoApprove {
    #[account(mut)]
    pub source: Account<TokenAccount>,
    pub delegate: UncheckedAccount,
    pub authority: Signer,
    pub token_program: Program<Token>,
}

#[derive(Accounts)]
pub struct DoRevoke {
    #[account(mut)]
    pub source: Account<TokenAccount>,
    pub authority: Signer,
    pub token_program: Program<Token>,
}

#[derive(Accounts)]
pub struct DoCloseAccount {
    #[account(mut)]
    pub account: Account<TokenAccount>,
    #[account(mut)]
    pub destination: UncheckedAccount,
    pub authority: Signer,
    pub token_program: Program<Token>,
}

#[derive(Accounts)]
pub struct ReadMint {
    pub mint: Account<Mint>,
}

#[derive(Accounts)]
pub struct ReadTokenAccount {
    pub token_account: Account<TokenAccount>,
}

#[derive(Accounts)]
pub struct CheckMintDecimals {
    #[account(mut, mint::decimals = 6)]
    pub mint: Account<Mint>,
}

#[derive(Accounts)]
pub struct CheckMintAuthority {
    pub expected: UncheckedAccount,
    #[account(mut, mint::authority = expected)]
    pub mint: Account<Mint>,
}

#[derive(Accounts)]
pub struct CheckTokenMint {
    pub mint: Account<Mint>,
    #[account(mut, token::mint = mint)]
    pub token_account: Account<TokenAccount>,
}

#[derive(Accounts)]
pub struct CheckTokenAuthority {
    pub expected: UncheckedAccount,
    #[account(mut, token::authority = expected)]
    pub token_account: Account<TokenAccount>,
}

#[derive(Accounts)]
pub struct CheckAta {
    pub authority: UncheckedAccount,
    pub mint: Account<Mint>,
    pub vault: Account<TokenAccount>,
}

// -- InterfaceAccount read-path structs --------------------------------------

#[derive(Accounts)]
pub struct ReadInterfaceMint {
    pub mint: InterfaceAccount<Mint>,
}

#[derive(Accounts)]
pub struct ReadInterfaceTokenAccount {
    pub token_account: InterfaceAccount<TokenAccount>,
}

// -- InterfaceAccount init-path structs --------------------------------------

#[derive(Accounts)]
pub struct InitInterfaceMint {
    #[account(mut)]
    pub payer: Signer,
    pub authority: UncheckedAccount,
    pub token_program: Program<Token>,
    #[account(
        init,
        payer = payer,
        mint::decimals = 6,
        mint::authority = authority,
        mint::token_program = token_program,
    )]
    pub mint: InterfaceAccount<Mint>,
    pub system_program: Program<System>,
}

#[derive(Accounts)]
pub struct InitInterfaceTokenAccount {
    #[account(mut)]
    pub payer: Signer,
    pub mint: InterfaceAccount<Mint>,
    pub authority: UncheckedAccount,
    pub token_program: Program<Token>,
    #[account(
        init,
        payer = payer,
        token::mint = mint,
        token::authority = authority,
        token::token_program = token_program,
    )]
    pub token_account: InterfaceAccount<TokenAccount>,
    pub system_program: Program<System>,
}

// -- Namespaced-constraint structs on InterfaceAccount -----------------------

#[derive(Accounts)]
pub struct CheckInterfaceTokenMint {
    pub mint: InterfaceAccount<Mint>,
    #[account(mut, token::mint = mint)]
    pub token_account: InterfaceAccount<TokenAccount>,
}

#[derive(Accounts)]
pub struct CheckInterfaceTokenAuthority {
    pub expected: UncheckedAccount,
    #[account(mut, token::authority = expected)]
    pub token_account: InterfaceAccount<TokenAccount>,
}

#[derive(Accounts)]
pub struct CheckInterfaceTokenProgram {
    pub token_program: UncheckedAccount,
    #[account(mut, token::token_program = token_program)]
    pub token_account: InterfaceAccount<TokenAccount>,
}

#[derive(Accounts)]
pub struct CheckInterfaceMintAuthority {
    pub expected: UncheckedAccount,
    #[account(mut, mint::authority = expected)]
    pub mint: InterfaceAccount<Mint>,
}

#[derive(Accounts)]
pub struct CheckInterfaceMintFreezeAuthority {
    pub expected: UncheckedAccount,
    #[account(mut, mint::freeze_authority = expected)]
    pub mint: InterfaceAccount<Mint>,
}

#[derive(Accounts)]
pub struct CheckInterfaceMintDecimals {
    #[account(mut, mint::decimals = 6)]
    pub mint: InterfaceAccount<Mint>,
}

#[derive(Accounts)]
pub struct CheckInterfaceMintTokenProgram {
    pub token_program: UncheckedAccount,
    #[account(mut, mint::token_program = token_program)]
    pub mint: InterfaceAccount<Mint>,
}

// -- Extension-reader structs ------------------------------------------------
//
// Each Token-2022 extension reader has its own Accounts struct so the
// struct's `#[instruction(...)]` list matches its handler's extra-args
// signature exactly. The single-deser contract (one Borsh parse of
// `ix_data`, shared between constraint validation and the handler) only
// works when each (accounts struct, handler) pair agrees on the wire
// shape — so handlers that previously shared `ReadMintExtension` /
// `ReadTokenAccountExtension` with divergent args get their own struct.

#[derive(Accounts)]
#[instruction(expected_bps: u16)]
pub struct ReadTransferFeeConfig {
    pub mint: InterfaceAccount<Mint>,
}

#[derive(Accounts)]
#[instruction(expected_authority: Address, expected_metadata: Address)]
pub struct ReadMetadataPointer {
    pub mint: InterfaceAccount<Mint>,
}

#[derive(Accounts)]
#[instruction(expected_program_id: Address)]
pub struct ReadTransferHook {
    pub mint: InterfaceAccount<Mint>,
}

#[derive(Accounts)]
#[instruction(expected_authority: Address)]
pub struct ReadMintCloseAuthority {
    pub mint: InterfaceAccount<Mint>,
}

#[derive(Accounts)]
#[instruction(expected_delegate: Address)]
pub struct ReadPermanentDelegate {
    pub mint: InterfaceAccount<Mint>,
}

#[derive(Accounts)]
#[instruction(expected_withheld: u64)]
pub struct ReadTransferFeeAmount {
    pub token_account: InterfaceAccount<TokenAccount>,
}

#[derive(Accounts)]
#[instruction(expected_transferring: u8)]
pub struct ReadTransferHookAccount {
    pub token_account: InterfaceAccount<TokenAccount>,
}
