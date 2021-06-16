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
/// | `#[account(signer)]` | On raw `AccountInfo` structs. | Checks the given account signed the transaction. |
/// | `#[account(mut)]` | On `AccountInfo`, `ProgramAccount` or `CpiAccount` structs. | Marks the account as mutable and persists the state transition. |
/// | `#[account(init)]` | On `ProgramAccount` structs. | Marks the account as being initialized, skipping the account discriminator check. When using `init`, a `rent` `Sysvar` must be present in the `Accounts` struct. |
/// | `#[account(close = <target>)]` | On `ProgramAccount` and `Loader` structs. | Marks the account as being closed at the end of the instruction's execution, sending the rent exemption lamports to the specified <target>. |
/// | `#[account(belongs_to = <target>)]` | On `ProgramAccount` or `CpiAccount` structs | Checks the `target` field on the account matches the `target` field in the struct deriving `Accounts`. |
/// | `#[account(has_one = <target>)]` | On `ProgramAccount` or `CpiAccount` structs | Semantically different, but otherwise the same as `belongs_to`. |
/// | `#[account(seeds = [<seeds>])]` | On `AccountInfo` structs | Seeds for the program derived address an `AccountInfo` struct represents. |
/// | `#[account(constraint = <expression>)]` | On any type deriving `Accounts` | Executes the given code as a constraint. The expression should evaluate to a boolean. |
/// | `#[account("<literal>")]` | Deprecated | Executes the given code literal as a constraint. The literal should evaluate to a boolean. |
/// | `#[account(rent_exempt = <skip>)]` | On `AccountInfo` or `ProgramAccount` structs | Optional attribute to skip the rent exemption check. By default, all accounts marked with `#[account(init)]` will be rent exempt, and so this should rarely (if ever) be used. Similarly, omitting `= skip` will mark the account rent exempt. |
/// | `#[account(executable)]` | On `AccountInfo` structs | Checks the given account is an executable program. |
/// | `#[account(state = <target>)]` | On `CpiState` structs | Checks the given state is the canonical state account for the target program. |
/// | `#[account(owner = <target>)]` | On `CpiState`, `CpiAccount`, and `AccountInfo` | Checks the account owner matches the target. |
/// | `#[account(associated = <target>, with? = <target>, payer? = <target>, space? = "<literal>")]` | On `ProgramAccount` | Whe `init` is provided, creates an associated program account at a program derived address. `associated` is the SOL address to create the account for. `with` is an optional association, for example, a `Mint` account in the SPL token program. `payer` is an optional account to pay for the account creation, defaulting to the `associated` target if none is given. `space` is an optional literal specifying how large the account is, defaulting to the account's serialized `Default::default` size (+ 8 for the account discriminator) if none is given. When creating an associated account, a `rent` `Sysvar` and `system_program` `AccountInfo` must be present in the `Accounts` struct. When `init` is not provided, then ensures the given associated account has the expected address, defined by the program and the given seeds. |
// TODO: How do we make the markdown render correctly without putting everything
//       on absurdly long lines?
#[proc_macro_derive(Accounts, attributes(account, instruction))]
pub fn derive_anchor_deserialize(item: TokenStream) -> TokenStream {
    parse_macro_input!(item as anchor_syn::AccountsStruct)
        .to_token_stream()
        .into()
}
