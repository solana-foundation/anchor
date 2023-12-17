extern crate proc_macro;

use quote::ToTokens;
use syn::parse_macro_input;

/// The `#[program]` attribute defines the module containing all instruction
/// handlers defining all entries into a Solana program.
#[proc_macro_attribute]
pub fn program(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    parse_macro_input!(input as anchor_syn::Program)
        .to_token_stream()
        .into()
}

/// The `#[interface]` attribute is used to mark an instruction as belonging
/// to an interface implementation, thus transforming its discriminator to the
/// proper bytes for that interface instruction.
///
/// # Example
///
/// ```rust,ignore
/// use anchor_lang::prelude::*;
///
/// // SPL Transfer Hook Interface: `Execute` instruction.
/// //
/// // This instruction is invoked by Token-2022 when a transfer occurs,
/// // if a mint has specified this program as its transfer hook.
/// #[interface(spl_transfer_hook_interface::execute)]
/// pub fn execute_transfer(ctx: Context<Execute>, amount: u64) -> Result<()> {
///     // Check that all extra accounts were provided
///     let data = ctx.accounts.extra_metas_account.try_borrow_data()?;
///     ExtraAccountMetaList::check_account_infos::<ExecuteInstruction>(
///         &ctx.accounts.to_account_infos(),
///         &TransferHookInstruction::Execute { amount }.pack(),
///         &ctx.program_id,
///         &data,
///     )?;
///
///     // Or maybe perform some custom logic
///     if ctx.accounts.token_metadata.mint != ctx.accounts.token_account.mint {
///         return Err(ProgramError::IncorrectAccount);
///     }
///
///     Ok(())
/// }
/// ```
#[cfg(feature = "interface-instructions")]
#[proc_macro_attribute]
pub fn interface(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // This macro itself is a no-op, but must be defined as a proc-macro
    // attribute to be used on a function as the `#[interface]` attribute.
    //
    // The `#[program]` macro will detect this attribute and transform the
    // discriminator.
    input
}
