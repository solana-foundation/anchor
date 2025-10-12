//! Doctest examples for #[account] macro

use anchor_lang::prelude::*;

/// Example 1: The Problem - Why regular #[account] fails in doctests
///
/// This example intentionally fails to demonstrate the issue:
///
/// ```compile_fail
/// use anchor_lang::prelude::*;
///
/// declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
///
/// #[account]
/// struct MyAccount {
///     data: u64,
/// }
/// ```
///
/// **Why it fails:**
/// - Doctests compile each test in a separate submodule
/// - The `#[account]` macro generates `crate::ID` in the Owner trait
/// - But `declare_id!` creates `ID` in the local submodule, not at `crate::ID`
/// - Result: "cannot find value `ID` in the crate root"
pub struct ProblemDemonstration;

/// Example 2: Solution 1 - Using `id` parameter
///
/// Use the `id` parameter to specify the program ID explicitly.
/// This enables using `declare_id!` in doctests while generating the Owner trait.
///
/// ```
/// use anchor_lang::prelude::*;
///
/// declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
///
/// #[account(id = ID)]  // Use ID (local), not crate::ID
/// struct MyAccount {
///     data: u64,
///     owner: Pubkey,
/// }
///
/// // The Owner trait is generated with the declared ID
/// assert_eq!(MyAccount::owner(), ID);
///
/// // The Discriminator trait is also implemented
/// assert_eq!(MyAccount::DISCRIMINATOR.len(), 8);
/// ```
pub struct SolutionWithIdParameter;

/// Example 3: Solution 2 - Using namespace (Alternative - Simpler)
///
/// Use a namespace string to skip Owner trait generation.
/// Simpler but the Owner trait won't be available.
///
/// ```
/// use anchor_lang::prelude::*;
///
/// declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
///
/// #[account("doctest")]  // Namespace skips Owner trait
/// struct NamespacedAccount {
///     owner: Pubkey,
///     balance: u64,
/// }
///
/// // The Discriminator trait is still implemented
/// assert_eq!(NamespacedAccount::DISCRIMINATOR.len(), 8);
///
/// // Note: Owner trait is NOT generated when using a namespace
/// // NamespacedAccount::owner() would not compile
/// ```
pub struct SolutionWithNamespace;

/// Example 4: Comparing id vs namespace approaches
///
/// This shows the difference between the two solutions.
///
/// ```
/// use anchor_lang::prelude::*;
///
/// declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
///
/// #[account(id = ID)]  // Uses "account" namespace, HAS Owner trait
/// struct WithId {
///     balance: u64,
/// }
///
/// #[account("custom")]  // Uses "custom" namespace, NO Owner trait
/// struct WithNamespace {
///     authority: Pubkey,
/// }
///
/// // WithId has Owner trait
/// assert_eq!(WithId::owner(), ID);
///
/// // WithNamespace does NOT have Owner trait
/// // WithNamespace::owner() would not compile
///
/// // Different discriminators due to different namespaces
/// assert_ne!(WithId::DISCRIMINATOR, WithNamespace::DISCRIMINATOR);
/// ```
pub struct ComparisonExample;

#[account]
pub struct A {
    pub counter: u64,
}

#[account(id = system_program::ID)]
pub struct B {
    pub counter: u64,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a_has_owner() {
        // Regular accounts in the actual program use crate::ID automatically
        assert_eq!(A::owner(), crate::ID);
    }

    #[test]
    fn test_a_discriminator() {
        assert_eq!(A::DISCRIMINATOR.len(), 8);
    }

    #[test]
    fn test_b_has_owner() {
        assert_ne!(B::owner(), crate::ID);
    }

    #[test]
    fn test_b_discriminator() {
        assert_eq!(B::DISCRIMINATOR.len(), 8);
    }
}
