//! Program account alias backed by generic `Account`.

/// Type validating that the account is the given Program
///
/// The type has a `programdata_address` function that will return `Option::Some`
/// if the program is owned by the [`BPFUpgradeableLoader`](https://docs.rs/solana-program/latest/solana_program/bpf_loader_upgradeable/index.html)
/// which will contain the `programdata_address` property of the `Program` variant of the [`UpgradeableLoaderState`](https://docs.rs/solana-loader-v3-interface/latest/solana_loader_v3_interface/state/enum.UpgradeableLoaderState.html) enum.
///
/// # Table of Contents
/// - [Basic Functionality](#basic-functionality)
/// - [Generic Program Validation](#generic-program-validation)
/// - [Out of the Box Types](#out-of-the-box-types)
///
/// # Basic Functionality
///
/// For `Program<'info, T>` where T implements Id:
/// Checks:
///
/// - `account_info.key == expected_program`
/// - `account_info.executable == true`
///
/// # Generic Program Validation
///
/// For `Program<'info>` (without type parameter):
/// - Only checks: `account_info.executable == true`
/// - Use this when you only need to verify that an address is executable,
///   without validating against a specific program ID.
///
/// # Example
/// ```ignore
/// #[program]
/// mod my_program {
///     fn set_admin_settings(...){...}
/// }
///
/// #[account]
/// #[derive(Default)]
/// pub struct AdminSettings {
///     ...
/// }
///
/// #[derive(Accounts)]
/// pub struct SetAdminSettings<'info> {
///     #[account(mut, seeds = [b"admin"], bump)]
///     pub admin_settings: Account<'info, AdminSettings>,
///     #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
///     pub program: Program<'info, MyProgram>,
///     #[account(constraint = program_data.upgrade_authority_address == Some(authority.key()))]
///     pub program_data: Account<'info, ProgramData>,
///     pub authority: Signer<'info>,
/// }
/// ```
/// The given program has a function with which the upgrade authority can set admin settings.
///
/// The required constraints are as follows:
///
/// - `program` is the account of the program itself.
///   Its constraint checks that `program_data` is the account that contains the program's upgrade authority.
///   Implicitly, this checks that `program` is a BPFUpgradeable program (`program.programdata_address()?`
///   will be `None` if it's not).
/// - `program_data`'s constraint checks that its upgrade authority is the `authority` account.
/// - Finally, `authority` needs to sign the transaction.
///
/// ## Generic Program Example
/// ```ignore
/// #[derive(Accounts)]
/// pub struct ValidateExecutableProgram<'info> {
///     // Only validates that the provided account is executable
///     pub any_program: Program<'info>,
///     pub authority: Signer<'info>,
/// }
/// ```
///
/// # Out of the Box Types
///
/// Between the [`anchor_lang`](https://docs.rs/anchor-lang/latest/anchor_lang) and [`anchor_spl`](https://docs.rs/anchor_spl/latest/anchor_spl) crates,
/// the following `Program` types are provided out of the box:
///
/// - [`System`](https://docs.rs/anchor-lang/latest/anchor_lang/struct.System.html)
/// - [`AssociatedToken`](https://docs.rs/anchor-spl/latest/anchor_spl/associated_token/struct.AssociatedToken.html)
/// - [`Token`](https://docs.rs/anchor-spl/latest/anchor_spl/token/struct.Token.html)
///
pub type Program<'info, T = crate::accounts::account::AnyProgram> =
    crate::accounts::account::Account<'info, crate::accounts::account::Program<T>>;
