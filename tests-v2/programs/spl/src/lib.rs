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
        mint::{self, Mint},
        token::{self, cpi as token_cpi, TokenAccount},
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
#[instruction(amount: u64)]
pub struct DoMintTo {
    #[account(mut)]
    pub mint: Account<Mint>,
    #[account(mut)]
    pub to: Account<TokenAccount>,
    pub authority: Signer,
    pub token_program: Program<Token>,
}

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct DoTransfer {
    #[account(mut)]
    pub from: Account<TokenAccount>,
    #[account(mut)]
    pub to: Account<TokenAccount>,
    pub authority: Signer,
    pub token_program: Program<Token>,
}

#[derive(Accounts)]
#[instruction(amount: u64, decimals: u8)]
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
#[instruction(amount: u64)]
pub struct DoBurn {
    #[account(mut)]
    pub account: Account<TokenAccount>,
    #[account(mut)]
    pub mint: Account<Mint>,
    pub authority: Signer,
    pub token_program: Program<Token>,
}

#[derive(Accounts)]
#[instruction(amount: u64)]
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
