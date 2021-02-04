extern crate proc_macro;

use anchor_syn::codegen::accounts as accounts_codegen;
use anchor_syn::parser::accounts as accounts_parser;
use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Implements an [`Accounts`](./trait.Accounts.html) deserializer on the given
/// struct, applying any constraints specified via inert `#[account(..)]`
/// attributes upon deserialization.
///
/// # Example
///
/// ```
/// #[derive(Accounts)]
/// pub struct Auth<'info> {
///     #[account(mut, has_one = authority)]
///     pub data: ProgramAccount<'info, MyData>,
///     #[account(signer)]
///     pub authority: AccountInfo<'info>,
/// }
///
/// #[account]
/// pub struct MyData {
///   authority: Pubkey,
///   protected_data: u64,
/// }
/// ```
///
/// Here, any instance of the `Auth` struct created via
/// [`try_accounts`](trait.Accounts.html#tymethod.try_accounts) is guaranteed
/// to have been both
///
/// * Signed by `authority`.
/// * Checked that `&data.authority == authority.key`.
///
/// The full list of available attributes is as follows.
///
/// | Attribute | Location | Description |
/// |:--|:--|:--|
/// | `#[account(signer)]` | On raw `AccountInfo` structs. | Checks the given account signed the transaction. |
/// | `#[account(mut)]` | On `AccountInfo`, `ProgramAccount` or `CpiAccount` structs. | Marks the account as mutable and persists the state transition. |
/// | `#[account(init)]` | On `ProgramAccount` structs. | Marks the account as being initialized, skipping the account discriminator check. |
/// | `#[account(belongs_to = <target>)]` | On `ProgramAccount` or `CpiAccount` structs | Checks the `target` field on the account matches the `target` field in the struct deriving `Accounts`. |
/// | `#[account(has_one = <target>)]` | On `ProgramAccount` or `CpiAccount` structs | Semantically different, but otherwise the same as `belongs_to`. |
/// | `#[account(seeds = [<seeds>])]` | On `AccountInfo` structs | Seeds for the program derived address an `AccountInfo` struct represents. |
/// | `#[account("<literal>")]` | On any type deriving `Accounts` | Executes the given code literal as a constraint. The literal should evaluate to a boolean. |
/// | `#[account(rent_exempt = <skip>)]` | On `AccountInfo` or `ProgramAccount` structs | Optional attribute to skip the rent exemption check. By default, all accounts marked with `#[account(init)]` will be rent exempt, and so this should rarely (if ever) be used. Similarly, omitting `= skip` will mark the account rent exempt. |
#[proc_macro_derive(Accounts, attributes(account))]
pub fn derive_anchor_deserialize(item: TokenStream) -> TokenStream {
    let strct = parse_macro_input!(item as syn::ItemStruct);
    let tts = accounts_codegen::generate(accounts_parser::parse(&strct));
    proc_macro::TokenStream::from(tts)
}
