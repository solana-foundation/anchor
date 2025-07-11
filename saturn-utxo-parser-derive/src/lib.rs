use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod codegen;
mod ir;
mod parse;
mod validate;

/// Derive macro that generates an implementation of [`TryFromUtxos`] for a
/// struct, providing declarative parsing and validation of
/// [`saturn_bitcoin_transactions::utxo_info::UtxoInfo`] inputs.
///
/// This macro enables you to define strongly-typed structures that automatically
/// parse and validate UTXO inputs according to your specification, eliminating
/// boilerplate code and reducing the chance of errors.
///
/// # How it works
///
/// Each field of the annotated struct is matched against the slice supplied to
/// `try_utxos` according to the field's *type* and optional `#[utxo(..)]`
/// *attribute*. Matched UTXOs are removed from consideration; if any inputs
/// remain unconsumed, or a validation check fails, the generated
/// method returns an appropriate [`ProgramError`]:
///
/// ```ignore
/// // Mandatory UTXO could not be found
/// ProgramError::Custom(ErrorCode::MissingRequiredUtxo.into())
///
/// // There are leftover inputs not matched by any field
/// ProgramError::Custom(ErrorCode::UnexpectedExtraUtxos.into())
///
/// // Predicate checks failed
/// ProgramError::Custom(ErrorCode::InvalidUtxoValue.into())
/// ProgramError::Custom(ErrorCode::InvalidRunesPresence.into())
/// ProgramError::Custom(ErrorCode::InvalidRuneId.into())
/// ProgramError::Custom(ErrorCode::InvalidRuneAmount.into())
/// ```
///
/// # Supported field types
///
/// | Rust type                               | Behaviour                                              |
/// | --------------------------------------- | ------------------------------------------------------ |
/// | `UtxoInfo`                               | Exactly one matching UTXO **must** be present.         |
/// | `Option<UtxoInfo>`                       | Zero or one matching UTXO may be present.              |
/// | `[UtxoInfo; N]`                          | Exactly *N* matching UTXOs must be present.            |
/// | `Vec<UtxoInfo>` **(see `rest`)**         | Variable-length list capturing remaining UTXOs.        |
///
/// A `Vec` field **must** be annotated with the `rest` flag, otherwise the
/// compilation will fail.
///
/// # `#[utxo(..)]` attribute
///
/// The attribute accepts a comma-separated list of *flags* and *key/value*
/// pairs that narrow the search predicate for the associated field:
///
/// ## Flags
///   * `rest` – mark a `Vec` field as the catch-all container receiving any
///     inputs not matched by earlier fields.
///
/// ## Key/Value Pairs
///   * `value = <expr>` – match only UTXOs whose `value` (in satoshis) is equal
///     to the given expression.
///   * `runes = "none" | "some" | "any"` – constrain presence of runes:
///       * `"none"` – assert that no runes are present.
///       * `"some"` – assert that at least one rune is present.
///       * `"any"` – do not check runes (default).
///   * `rune_id = <expr>` – match only UTXOs that contain the specified rune
///     id. May be combined with `rune_amount` for an exact match.
///   * `rune_amount = <expr>` – If `rune_id` is also provided, require the UTXO
///     to hold exactly this amount of the given rune. Otherwise require the
///     *total* rune amount inside the UTXO to equal the expression.
///   * `anchor = <ident>` – Expect identifier that refers to a field in the Accounts struct. If `runes` is **omitted** on an anchored field, it is implicitly treated as `runes = "none"` for backward compatibility.
///
/// The predicate generated from these parameters is applied to each candidate
/// UTXO until a match is found.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust,ignore
/// use saturn_utxo_parser::{UtxoParser, TryFromUtxos};
/// use saturn_bitcoin_transactions::utxo_info::UtxoInfo;
///
/// #[derive(UtxoParser)]
/// struct SimpleSwap {
///     // UTXO paying the on-chain fee.
///     #[utxo(value = 10_000, runes = "none")]
///     fee: UtxoInfo,
///
///     // Optional rune deposit, any amount.
///     #[utxo(runes = "some")]
///     deposit: Option<UtxoInfo>,
///
///     // Capture all remaining inputs.
///     #[utxo(rest)]
///     others: Vec<UtxoInfo>,
/// }
/// ```
///
/// ## Array Fields
///
/// ```rust,ignore
/// use saturn_utxo_parser::{UtxoParser, TryFromUtxos};
/// use saturn_bitcoin_transactions::utxo_info::UtxoInfo;
///
/// #[derive(UtxoParser)]
/// struct MultiInput {
///     // Exactly 3 UTXOs with specific value
///     #[utxo(value = 5_000)]
///     inputs: [UtxoInfo; 3],
/// }
/// ```
///
/// ## Rune-specific Matching
///
/// ```rust,ignore
/// use saturn_utxo_parser::{UtxoParser, TryFromUtxos};
/// use saturn_bitcoin_transactions::utxo_info::UtxoInfo;
///
/// #[derive(UtxoParser)]
/// struct RuneTransfer {
///     // Exact rune ID and amount
///     #[utxo(rune_id = my_rune_id, rune_amount = 1000)]
///     specific_rune: UtxoInfo,
///
///     // Any UTXO with exactly 500 total runes
///     #[utxo(rune_amount = 500)]
///     any_rune_500: UtxoInfo,
/// }
/// ```
///
/// # Important Notes
///
/// - Field order matters: UTXOs are matched in the order fields appear in the struct
/// - Each UTXO can only be matched once
/// - The `rest` field (if present) should typically be the last field
/// - All expressions in attributes are evaluated in the context where the macro is used
///
/// [`TryFromUtxos`]: crate::TryFromUtxos
/// [`ProgramError`]: arch_program::program_error::ProgramError
#[proc_macro_derive(UtxoParser, attributes(utxo, utxo_accounts))]
pub fn derive_utxo_parser(item: TokenStream) -> TokenStream {
    // Parse the incoming tokens into `syn::DeriveInput` first.
    let input = parse_macro_input!(item as DeriveInput);

    // 1) Convert to internal IR
    let ir = match parse::derive_input_to_ir(&input) {
        Ok(ir) => ir,
        Err(e) => return e.to_compile_error().into(),
    };

    // 2) Run semantic validation
    if let Err(e) = validate::check(&ir) {
        return e.to_compile_error().into();
    }

    // 3) Generate final implementation
    let expanded = codegen::expand(&ir);

    TokenStream::from(expanded)
}
