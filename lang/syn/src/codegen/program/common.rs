use crate::IxArg;
use heck::{ToLowerCamelCase, ToPascalCase};
use quote::quote;
use regex::Regex;
use std::sync::OnceLock;

// Namespace for calculating instruction sighash signatures for any instruction
// not affecting program state.
pub const SIGHASH_GLOBAL_NAMESPACE: &str = "global";

// We don't technically use sighash, because the input arguments aren't given.
// Rust doesn't have method overloading so no need to use the arguments.
// However, we do namespace methods in the preeimage so that we can use
// different traits with the same method name.
pub fn sighash(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{namespace}:{name}");

    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(&crate::hash::hash(preimage.as_bytes()).to_bytes()[..8]);
    sighash
}

pub fn gen_discriminator(namespace: &str, name: impl ToString) -> proc_macro2::TokenStream {
    let discriminator = sighash(namespace, name.to_string().as_str());
    format!("&{discriminator:?}").parse().unwrap()
}

pub fn generate_ix_variant(name: &str, args: &[IxArg]) -> proc_macro2::TokenStream {
    let ix_arg_names: Vec<&syn::Ident> = args.iter().map(|arg| &arg.name).collect();
    let ix_name_camel = generate_ix_variant_name(name);

    if args.is_empty() {
        quote! {
            #ix_name_camel
        }
    } else {
        quote! {
            #ix_name_camel {
                #(#ix_arg_names),*
            }
        }
    }
}

pub fn generate_ix_variant_name(name: &str) -> proc_macro2::TokenStream {
    let n = harmonized_pascal_case(name);
    n.parse().unwrap()
}

static NUMBER_LETTER_PATTERN: OnceLock<Regex> = OnceLock::new();

/// Harmonized camelCase conversion that handles number+letter patterns consistently
/// with JavaScript's camelcase library behavior.
///
/// This function normalizes Rust snake_case identifiers to camelCase following
/// a specific convention for number+letter transitions:
/// - `a1b_receive` → `a1BReceive` (digit followed by letter gets capitalized)
/// - `test2var_function` → `test2VarFunction`
/// - `sha3_sum` → `sha3Sum`
///
/// **Note for TypeScript users**: When calling instructions with number+letter
/// patterns from TypeScript, use the harmonized camelCase form:
/// ```typescript
/// program.methods.a1BReceive()  // ✓ correct
/// program.methods.a1bReceive()  // ✗ will not work
/// ```
pub fn harmonized_camel_case(input: &str) -> String {
    let pattern = NUMBER_LETTER_PATTERN.get_or_init(|| Regex::new(r"(\d)([a-zA-Z])").unwrap());

    let result = input.to_lower_camel_case(); // gives proper camelCase

    // Fix number+letter patterns
    pattern
        .replace_all(&result, |caps: &regex::Captures| {
            format!("{}{}", &caps[1], caps[2].to_uppercase())
        })
        .to_string()
}

/// Harmonized PascalCase conversion for Rust type identifiers.
/// Mirrors `harmonized_camel_case` rules but produces PascalCase, so generated
/// Rust struct/enum names follow Rust naming conventions and avoid lints.
pub fn harmonized_pascal_case(input: &str) -> String {
    let pattern = NUMBER_LETTER_PATTERN.get_or_init(|| Regex::new(r"(\d)([a-zA-Z])").unwrap());

    let result = input.to_pascal_case(); // base PascalCase

    pattern
        .replace_all(&result, |caps: &regex::Captures| {
            format!("{}{}", &caps[1], caps[2].to_uppercase())
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harmonized_camel_case() {
        assert_eq!(harmonized_camel_case("a1b_receive"), "a1BReceive");
        assert_eq!(
            harmonized_camel_case("test2var_function"),
            "test2VarFunction"
        );
        assert_eq!(harmonized_camel_case("my_3x_param"), "my3XParam");
        assert_eq!(harmonized_camel_case("normal_function"), "normalFunction");
        assert_eq!(harmonized_camel_case("initialize"), "initialize");

        // Multiple digits and transitions
        assert_eq!(harmonized_camel_case("a12bc_test"), "a12BcTest");
        assert_eq!(harmonized_camel_case("func2var3thing"), "func2Var3Thing");
        assert_eq!(
            harmonized_camel_case("test123abc456def"),
            "test123Abc456Def"
        );

        // Leading digits
        assert_eq!(harmonized_camel_case("2factor_auth"), "2FactorAuth");
        assert_eq!(harmonized_camel_case("3d_render"), "3DRender");

        // Acronyms and algorithms with digits
        assert_eq!(harmonized_camel_case("sha3_sum"), "sha3Sum");
        assert_eq!(harmonized_camel_case("md5_hash"), "md5Hash");
        assert_eq!(harmonized_camel_case("base64_encode"), "base64Encode");

        // Edge cases
        assert_eq!(harmonized_camel_case("a1"), "a1");
        assert_eq!(harmonized_camel_case("1a"), "1A");
        assert_eq!(harmonized_camel_case("a1b2c3d"), "a1B2C3D");
    }

    #[test]
    fn test_harmonized_pascal_case() {
        assert_eq!(harmonized_pascal_case("a1b_receive"), "A1BReceive");
        assert_eq!(
            harmonized_pascal_case("test2var_function"),
            "Test2VarFunction"
        );
        assert_eq!(harmonized_pascal_case("initialize"), "Initialize");
        assert_eq!(harmonized_pascal_case("my_3x_param"), "My3XParam");
    }
}
