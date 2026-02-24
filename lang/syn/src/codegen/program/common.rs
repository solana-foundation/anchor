use crate::IxArg;
use anyhow::Result;
use quote::quote;

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

pub fn generate_ix_variant(name: &str, args: &[IxArg]) -> Result<proc_macro2::TokenStream> {
    let ix_arg_names: Vec<&syn::Ident> = args.iter().map(|arg| &arg.name).collect();
    let ix_name_camel = generate_ix_variant_name(name)?;

    let variant = if args.is_empty() {
        quote! {
            #ix_name_camel
        }
    } else {
        quote! {
            #ix_name_camel {
                #(#ix_arg_names),*
            }
        }
    };
    Ok(variant)
}

pub fn generate_ix_variant_name(name: &str) -> Result<syn::Ident> {
    Ok(syn::parse_str(&harmonized_pascal_case(name))?)
}

/// Converts snake_case, SCREAMING_SNAKE_CASE, or PascalCase to camelCase/PascalCase
/// with consistent digit-letter handling.
///
/// - First letter case is determined by `pascal` parameter
/// - Letters following underscores are capitalized (snake_case handling)
/// - Letters following digits are capitalized (e.g., a1b_receive → a1BReceive)
/// - For inputs with underscores (snake_case/SCREAMING_SNAKE_CASE), letters are lowercased
///   except at word boundaries (MY_CONST → myConst)
/// - For inputs without underscores (PascalCase), internal capitalization is preserved
///   (DummyA → dummyA)
///
/// This ensures consistent naming between Rust IDL generation and TypeScript clients.
fn convert_case(input: &str, pascal: bool) -> String {
    let mut result = String::with_capacity(input.len());
    let mut capitalize_next = pascal;
    let mut prev_was_digit = false;
    let mut is_first = true;
    // If input has underscores, treat as snake_case/SCREAMING_SNAKE_CASE and lowercase letters
    let has_underscore = input.contains('_');

    for c in input.chars() {
        if c == '_' {
            capitalize_next = true;
            prev_was_digit = false;
        } else if c.is_ascii_digit() {
            result.push(c);
            prev_was_digit = true;
            is_first = false;
        } else if c.is_ascii_alphabetic() {
            if is_first {
                // First letter: uppercase for PascalCase, lowercase for camelCase
                if pascal {
                    result.push(c.to_ascii_uppercase());
                } else {
                    result.push(c.to_ascii_lowercase());
                }
            } else if capitalize_next || prev_was_digit {
                // After underscore or digit, always capitalize
                result.push(c.to_ascii_uppercase());
            } else if has_underscore {
                // For snake_case/SCREAMING_SNAKE_CASE, lowercase within words
                result.push(c.to_ascii_lowercase());
            } else {
                // For PascalCase (no underscores), preserve original case
                result.push(c);
            }
            capitalize_next = false;
            prev_was_digit = false;
            is_first = false;
        } else {
            result.push(c);
            capitalize_next = false;
            prev_was_digit = false;
            is_first = false;
        }
    }
    result
}

pub fn harmonized_camel_case(input: &str) -> String {
    convert_case(input, false)
}

pub fn harmonized_pascal_case(input: &str) -> String {
    convert_case(input, true)
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

        // Edge cases
        assert_eq!(harmonized_camel_case("2factor_auth"), "2FactorAuth");
        assert_eq!(harmonized_camel_case("sha3_sum"), "sha3Sum");

        // PascalCase inputs (preserve internal capitalization)
        assert_eq!(harmonized_camel_case("DummyA"), "dummyA");
        assert_eq!(harmonized_camel_case("Initialize"), "initialize");
        assert_eq!(harmonized_camel_case("CompositeUpdate"), "compositeUpdate");

        // SCREAMING_SNAKE_CASE inputs (constants)
        assert_eq!(harmonized_camel_case("MY_CONST"), "myConst");
        assert_eq!(harmonized_camel_case("BYTE_STR"), "byteStr");
        assert_eq!(harmonized_camel_case("BYTES_STR"), "bytesStr");
        assert_eq!(harmonized_camel_case("U8"), "u8");
        assert_eq!(harmonized_camel_case("I128"), "i128");
    }

    #[test]
    fn test_harmonized_pascal_case() {
        assert_eq!(harmonized_pascal_case("a1b_receive"), "A1BReceive");
        assert_eq!(
            harmonized_pascal_case("test2var_function"),
            "Test2VarFunction"
        );
        assert_eq!(harmonized_pascal_case("normal_function"), "NormalFunction");
        assert_eq!(harmonized_pascal_case("initialize"), "Initialize");
    }
}
