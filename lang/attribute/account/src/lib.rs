extern crate proc_macro;

#[cfg(feature = "idl-build")]
use anchor_syn::idl::build::*;
use quote::quote;
use syn::parse_macro_input;

mod id;

/// An attribute for a data structure representing a Solana account.
///
/// `#[account]` generates trait implementations for the following traits:
///
/// - [`AccountSerialize`](./trait.AccountSerialize.html)
/// - [`AccountDeserialize`](./trait.AccountDeserialize.html)
/// - [`AnchorSerialize`](./trait.AnchorSerialize.html)
/// - [`AnchorDeserialize`](./trait.AnchorDeserialize.html)
/// - [`Clone`](https://doc.rust-lang.org/std/clone/trait.Clone.html)
/// - [`Discriminator`](./trait.Discriminator.html)
/// - [`Owner`](./trait.Owner.html)
///
/// When implementing account serialization traits the first 8 bytes are
/// reserved for a unique account discriminator, self described by the first 8
/// bytes of the SHA256 of the account's Rust ident.
///
/// As a result, any calls to `AccountDeserialize`'s `try_deserialize` will
/// check this discriminator. If it doesn't match, an invalid account was given,
/// and the account deserialization will exit with an error.
///
/// # Zero Copy Deserialization
///
/// **WARNING**: Zero copy deserialization is an experimental feature. It's
/// recommended to use it only when necessary, i.e., when you have extremely
/// large accounts that cannot be Borsh deserialized without hitting stack or
/// heap limits.
///
/// ## Usage
///
/// To enable zero-copy-deserialization, one can pass in the `zero_copy`
/// argument to the macro as follows:
///
/// ```ignore
/// #[account(zero_copy)]
/// ```
///
/// This can be used to conveniently implement
/// [`ZeroCopy`](./trait.ZeroCopy.html) so that the account can be used
/// with [`AccountLoader`](./accounts/account_loader/struct.AccountLoader.html).
///
/// Other than being more efficient, the most salient benefit this provides is
/// the ability to define account types larger than the max stack or heap size.
/// When using borsh, the account has to be copied and deserialized into a new
/// data structure and thus is constrained by stack and heap limits imposed by
/// the BPF VM. With zero copy deserialization, all bytes from the account's
/// backing `RefCell<&mut [u8]>` are simply re-interpreted as a reference to
/// the data structure. No allocations or copies necessary. Hence the ability
/// to get around stack and heap limitations.
///
/// To facilitate this, all fields in an account must be constrained to be
/// "plain old  data", i.e., they must implement
/// [`Pod`](https://docs.rs/bytemuck/latest/bytemuck/trait.Pod.html). Please review the
/// [`safety`](https://docs.rs/bytemuck/latest/bytemuck/trait.Pod.html#safety)
/// section before using.
///
/// Using `zero_copy` requires adding the following to your `cargo.toml` file:
/// `bytemuck = { version = "1.4.0", features = ["derive", "min_const_generics"]}`
#[proc_macro_attribute]
pub fn account(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut namespace = "".to_string();
    let mut is_zero_copy = false;
    let mut unsafe_bytemuck = false;
    let args_str = args.to_string();
    let args: Vec<&str> = args_str.split(',').collect();
    if args.len() > 2 {
        panic!("Only two args are allowed to the account attribute.")
    }
    for arg in args {
        let ns = arg
            .to_string()
            .replace('\"', "")
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if ns == "zero_copy" {
            is_zero_copy = true;
            unsafe_bytemuck = false;
        } else if ns == "zero_copy(unsafe)" {
            is_zero_copy = true;
            unsafe_bytemuck = true;
        } else {
            namespace = ns;
        }
    }

    let account_strct = parse_macro_input!(input as syn::ItemStruct);
    let account_name = &account_strct.ident;
    let account_name_str = account_name.to_string();
    let (impl_gen, type_gen, where_clause) = account_strct.generics.split_for_impl();

    let discriminator: proc_macro2::TokenStream = {
        // Namespace the discriminator to prevent collisions.
        let discriminator_preimage = {
            // For now, zero copy accounts can't be namespaced.
            if namespace.is_empty() {
                format!("account:{account_name}")
            } else {
                format!("{namespace}:{account_name}")
            }
        };

        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(
            &anchor_syn::hash::hash(discriminator_preimage.as_bytes()).to_bytes()[..8],
        );
        format!("{discriminator:?}").parse().unwrap()
    };

    let owner_impl = {
        if namespace.is_empty() {
            quote! {
                #[automatically_derived]
                impl #impl_gen anchor_lang::Owner for #account_name #type_gen #where_clause {
                    fn owner() -> Pubkey {
                        crate::ID
                    }
                }
            }
        } else {
            quote! {}
        }
    };

    let unsafe_bytemuck_impl = {
        if unsafe_bytemuck {
            quote! {
                #[automatically_derived]
                unsafe impl #impl_gen anchor_lang::__private::bytemuck::Pod for #account_name #type_gen #where_clause {}
                #[automatically_derived]
                unsafe impl #impl_gen anchor_lang::__private::bytemuck::Zeroable for #account_name #type_gen #where_clause {}
            }
        } else {
            quote! {}
        }
    };

    let bytemuck_derives = {
        if !unsafe_bytemuck {
            quote! {
                #[zero_copy]
            }
        } else {
            quote! {
                #[zero_copy(unsafe)]
            }
        }
    };

    proc_macro::TokenStream::from({
        if is_zero_copy {
            quote! {
                #bytemuck_derives
                #account_strct

                #unsafe_bytemuck_impl

                #[automatically_derived]
                impl #impl_gen anchor_lang::ZeroCopy for #account_name #type_gen #where_clause {}

                #[automatically_derived]
                impl #impl_gen anchor_lang::Discriminator for #account_name #type_gen #where_clause {
                    const DISCRIMINATOR: [u8; 8] = #discriminator;
                }

                // This trait is useful for clients deserializing accounts.
                // It's expected on-chain programs deserialize via zero-copy.
                #[automatically_derived]
                impl #impl_gen anchor_lang::AccountDeserialize for #account_name #type_gen #where_clause {
                    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
                        if buf.len() < #discriminator.len() {
                            return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
                        }
                        let given_disc = &buf[..8];
                        if &#discriminator != given_disc {
                            return Err(anchor_lang::error!(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch).with_account_name(#account_name_str));
                        }
                        Self::try_deserialize_unchecked(buf)
                    }

                    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
                        let data: &[u8] = &buf[8..];
                        // Re-interpret raw bytes into the POD data structure.
                        let account = anchor_lang::__private::bytemuck::from_bytes(data);
                        // Copy out the bytes into a new, owned data structure.
                        Ok(*account)
                    }
                }

                #owner_impl
            }
        } else {
            quote! {
                #[derive(AnchorSerialize, AnchorDeserialize, Clone)]
                #account_strct

                #[automatically_derived]
                impl #impl_gen anchor_lang::AccountSerialize for #account_name #type_gen #where_clause {
                    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
                        if writer.write_all(&#discriminator).is_err() {
                            return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
                        }

                        if AnchorSerialize::serialize(self, writer).is_err() {
                            return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
                        }
                        Ok(())
                    }
                }

                #[automatically_derived]
                impl #impl_gen anchor_lang::AccountDeserialize for #account_name #type_gen #where_clause {
                    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
                        if buf.len() < #discriminator.len() {
                            return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
                        }
                        let given_disc = &buf[..8];
                        if &#discriminator != given_disc {
                            return Err(anchor_lang::error!(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch).with_account_name(#account_name_str));
                        }
                        Self::try_deserialize_unchecked(buf)
                    }

                    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
                        let mut data: &[u8] = &buf[8..];
                        AnchorDeserialize::deserialize(&mut data)
                            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
                    }
                }

                #[automatically_derived]
                impl #impl_gen anchor_lang::Discriminator for #account_name #type_gen #where_clause {
                    const DISCRIMINATOR: [u8; 8] = #discriminator;
                }

                #owner_impl
            }
        }
    })
}

#[proc_macro_derive(ZeroCopyAccessor, attributes(accessor))]
pub fn derive_zero_copy_accessor(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let account_strct = parse_macro_input!(item as syn::ItemStruct);
    let account_name = &account_strct.ident;
    let (impl_gen, ty_gen, where_clause) = account_strct.generics.split_for_impl();

    let fields = match &account_strct.fields {
        syn::Fields::Named(n) => n,
        _ => panic!("Fields must be named"),
    };
    let methods: Vec<proc_macro2::TokenStream> = fields
        .named
        .iter()
        .filter_map(|field: &syn::Field| {
            field
                .attrs
                .iter()
                .find(|attr| anchor_syn::parser::tts_to_string(&attr.path) == "accessor")
                .map(|attr| {
                    let mut tts = attr.tokens.clone().into_iter();
                    let g_stream = match tts.next().expect("Must have a token group") {
                        proc_macro2::TokenTree::Group(g) => g.stream(),
                        _ => panic!("Invalid syntax"),
                    };
                    let accessor_ty = match g_stream.into_iter().next() {
                        Some(token) => token,
                        _ => panic!("Missing accessor type"),
                    };

                    let field_name = field.ident.as_ref().unwrap();

                    let get_field: proc_macro2::TokenStream =
                        format!("get_{field_name}").parse().unwrap();
                    let set_field: proc_macro2::TokenStream =
                        format!("set_{field_name}").parse().unwrap();

                    quote! {
                        pub fn #get_field(&self) -> #accessor_ty {
                            anchor_lang::__private::ZeroCopyAccessor::get(&self.#field_name)
                        }
                        pub fn #set_field(&mut self, input: &#accessor_ty) {
                            self.#field_name = anchor_lang::__private::ZeroCopyAccessor::set(input);
                        }
                    }
                })
        })
        .collect();
    proc_macro::TokenStream::from(quote! {
        #[automatically_derived]
        impl #impl_gen #account_name #ty_gen #where_clause {
            #(#methods)*
        }
    })
}

/// A data structure that can be used as an internal field for a zero copy
/// deserialized account, i.e., a struct marked with `#[account(zero_copy)]`.
///
/// `#[zero_copy]` is just a convenient alias for
///
/// ```ignore
/// #[derive(Copy, Clone)]
/// #[derive(bytemuck::Zeroable)]
/// #[derive(bytemuck::Pod)]
/// #[repr(C)]
/// struct MyStruct {...}
/// ```
#[proc_macro_attribute]
pub fn zero_copy(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut is_unsafe = false;
    for arg in args.into_iter() {
        match arg {
            proc_macro::TokenTree::Ident(ident) => {
                if ident.to_string() == "unsafe" {
                    // `#[zero_copy(unsafe)]` maintains the old behaviour
                    //
                    // ```ignore
                    // #[derive(Copy, Clone)]
                    // #[repr(packed)]
                    // struct MyStruct {...}
                    // ```
                    is_unsafe = true;
                } else {
                    // TODO: how to return a compile error with a span (can't return prase error because expected type TokenStream)
                    panic!("expected single ident `unsafe`");
                }
            }
            _ => {
                panic!("expected single ident `unsafe`");
            }
        }
    }

    let account_strct = parse_macro_input!(item as syn::ItemStruct);

    // Takes the first repr. It's assumed that more than one are not on the
    // struct.
    let attr = account_strct
        .attrs
        .iter()
        .find(|attr| anchor_syn::parser::tts_to_string(&attr.path) == "repr");

    let repr = match attr {
        // Users might want to manually specify repr modifiers e.g. repr(C, packed)
        Some(_attr) => quote! {},
        None => {
            if is_unsafe {
                quote! {#[repr(packed)]}
            } else {
                quote! {#[repr(C)]}
            }
        }
    };

    let mut has_pod_attr = false;
    let mut has_zeroable_attr = false;
    for attr in account_strct.attrs.iter() {
        let token_string = attr.tokens.to_string();
        if token_string.contains("bytemuck :: Pod") {
            has_pod_attr = true;
        }
        if token_string.contains("bytemuck :: Zeroable") {
            has_zeroable_attr = true;
        }
    }

    // Once the Pod derive macro is expanded the compiler has to use the local crate's
    // bytemuck `::bytemuck::Pod` anyway, so we're no longer using the privately
    // exported anchor bytemuck `__private::bytemuck`, so that there won't be any
    // possible disparity between the anchor version and the local crate's version.
    let pod = if has_pod_attr || is_unsafe {
        quote! {}
    } else {
        quote! {#[derive(::bytemuck::Pod)]}
    };
    let zeroable = if has_zeroable_attr || is_unsafe {
        quote! {}
    } else {
        quote! {#[derive(::bytemuck::Zeroable)]}
    };

    let ret = quote! {
        #[derive(anchor_lang::__private::ZeroCopyAccessor, Copy, Clone)]
        #repr
        #pod
        #zeroable
        #account_strct
    };

    #[cfg(feature = "idl-build")]
    {
        let no_docs = get_no_docs();
        let idl_build_impl = gen_idl_build_impl_for_struct(&account_strct, no_docs);
        return proc_macro::TokenStream::from(quote! {
            #ret
            #idl_build_impl
        });
    }

    #[allow(unreachable_code)]
    proc_macro::TokenStream::from(ret)
}

/// Defines the program's ID. This should be used at the root of all Anchor
/// based programs.
#[proc_macro]
pub fn declare_id(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let id = parse_macro_input!(input as id::Id);
    proc_macro::TokenStream::from(quote! {#id})
}
