#![cfg(feature = "idl-build")]

use anchor_syn::idl::impl_idl_build_struct;
use syn::{parse_quote, ItemStruct};

fn check_serialization(item: ItemStruct, expected: &str) {
    let stream = impl_idl_build_struct(&item);
    let output = stream.to_string();
    assert!(
        output.contains(expected),
        "Output did not contain expected serialization: '{}'. Got: '{}'",
        expected,
        output
    );
}

#[test]
fn test_bytemuck_unsafe_qualified() {
    check_serialization(
        parse_quote! {
            #[derive(bytemuck::unsafe_impl_pod)]
            struct Foo {}
        },
        "IdlSerialization :: BytemuckUnsafe",
    );
}

#[test]
fn test_bytemuck_unsafe_direct() {
    check_serialization(
        parse_quote! {
            #[derive(unsafe_impl_pod)]
            struct Foo {}
        },
        "IdlSerialization :: BytemuckUnsafe",
    );
}

#[test]
fn test_bytemuck_safe() {
    check_serialization(
        parse_quote! {
            #[derive(bytemuck::pod)]
            struct Foo {}
        },
        "IdlSerialization :: Bytemuck",
    );

    check_serialization(
        parse_quote! {
            #[derive(pod)]
            struct Foo {}
        },
        "IdlSerialization :: Bytemuck",
    );
}

#[test]
fn test_false_positive_prevention() {
    check_serialization(
        parse_quote! {
            #[derive(MyUnsafeMacro)]
            struct Foo {}
        },
        "IdlSerialization :: default",
    );
}
