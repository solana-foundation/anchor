use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

// This module uses precise-capture syntax (Rust 1.82+). rustc accepts it,
// but syn v1 (used by anchor-syn's CrateContext::parse) cannot parse it.
mod helper;

#[program]
pub mod idl_forward_ref {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.state.updates = Updates { count: 0 };
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = payer, space = 8 + 8)]
    pub state: Account<'info, State>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// ─── REPRODUCER ─────────────────────────────────────────────────────────────
//
// State is declared BEFORE Updates (forward reference).
//
// When AnchorSerialize's idl-build codegen processes `updates: Updates`,
// it calls gen_idl_type(Updates), which triggers CRATE_DATA_CACHE.get_or_init.
// That calls CrateContext::parse, which reads every .rs file including
// helper.rs. syn v1 fails on `impl use<T>` → "expected identifier".
//
// Before #4325: the parse failure was silently ignored (if let Ok(Ok(ctx))).
// After #4325:  the failure is surfaced as a hard error AND cached in a
//               static OnceLock, poisoning all subsequent type lookups.

#[account]
#[derive(InitSpace)]
pub struct State {
    pub updates: Updates, // forward reference: Updates is defined below
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize, InitSpace)]
pub struct Updates {
    pub count: u64,
}
