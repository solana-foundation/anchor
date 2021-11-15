extern crate proc_macro;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

/// Implements an [`Accounts`](./trait.Accounts.html) deserializer on the given
/// struct, applying any constraints specified via inert `#[account(..)]`
/// attributes upon deserialization.
///
/// # Example
///
/// ```ignore
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
/// | `#[account(signer)]`<br><br>`#[account(signer @ <custom_error>)]` | On raw `AccountInfo` structs. | Checks the given account signed the transaction. Custom errors are supported via `@`. |
/// | `#[account(mut)]`<br><br>`#[account(mut @ <custom_error>)]` | On `AccountInfo`, `ProgramAccount` or `CpiAccount` structs. | Marks the account as mutable and persists the state transition. Custom errors are supported via `@`. |
/// | `#[account(init)]` | On `ProgramAccount` structs. | Marks the account as being initialized, creating the account via the system program. |
/// | `#[account(init_if_needed)]` | On `ProgramAccount` structs. | Same as `init` but skip if already initialized. |
/// | `#[account(zero)]` | On `ProgramAccount` structs. | Asserts the account discriminator is zero. |
/// | `#[account(close = <target>)]` | On `ProgramAccount` and `Loader` structs. | Marks the account as being closed at the end of the instruction's execution, sending the rent exemption lamports to the specified <target>. |
/// | `#[account(has_one = <target>)]`<br><br>`#[account(has_one = <target> @ <custom_error>)]` | On `ProgramAccount` or `CpiAccount` structs | Checks the `target` field on the account matches the `target` field in the struct deriving `Accounts`. Custom errors are supported via `@`. |
/// | `#[account(seeds = [<seeds>], bump? = <target>, payer? = <target>, space? = <target>, owner? = <target>)]` | On `AccountInfo` structs | Seeds for the program derived address an `AccountInfo` struct represents. If bump is provided, then appends it to the seeds. On initialization, validates the given bump is the bump provided by `Pubkey::find_program_address`.|
/// | `#[account(constraint = <expression>)]`<br><br>`#[account(constraint = <expression> @ <custom_error>)]` | On any type deriving `Accounts` | Executes the given code as a constraint. The expression should evaluate to a boolean. Custom errors are supported via `@`. |
/// | `#[account("<literal>")]` | Deprecated | Executes the given code literal as a constraint. The literal should evaluate to a boolean. |
/// | `#[account(rent_exempt = <skip>)]` | On `AccountInfo` or `ProgramAccount` structs | Optional attribute to skip the rent exemption check. By default, all accounts marked with `#[account(init)]` will be rent exempt, and so this should rarely (if ever) be used. Similarly, omitting `= skip` will mark the account rent exempt. |
/// | `#[account(executable)]` | On `AccountInfo` structs | Checks the given account is an executable program. |
/// | `#[account(state = <target>)]` | On `CpiState` structs | Checks the given state is the canonical state account for the target program. |
/// | `#[account(owner = <target>)]`<br><br>`#[account(owner = <target> @ <custom_error>)]` | On `CpiState`, `CpiAccount`, and `AccountInfo` | Checks the account owner matches the target. Custom errors are supported via `@`. |
/// | `#[account(address = <pubkey>)]`<br><br>`#[account(address = <pubkey> @ <custom_error>)]` | On `AccountInfo` and `Account` | Checks the account key matches the pubkey. Custom errors are supported via `@`. |
// TODO: How do we make the markdown render correctly without putting everything
//       on absurdly long lines?
#[proc_macro_derive(Accounts, attributes(account, instruction))]
pub fn derive_anchor_deserialize(item: TokenStream) -> TokenStream {
    parse_macro_input!(item as anchor_syn::AccountsStruct)
        .to_token_stream()
        .into()
}
