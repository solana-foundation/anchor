extern crate proc_macro;

use quote::quote;
use syn::parse_macro_input;

/// A data structure representing a Solana account, implementing various traits:
///
/// - [`AccountSerialize`](./trait.AccountSerialize.html)
/// - [`AccountDeserialize`](./trait.AccountDeserialize.html)
/// - [`AnchorSerialize`](./trait.AnchorSerialize.html)
/// - [`AnchorDeserialize`](./trait.AnchorDeserialize.html)
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
/// with [`Loader`](./struct.Loader.html).
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
/// [`Pod`](../bytemuck/trait.Pod.html). Please review the
/// [`safety`](file:///home/armaniferrante/Documents/code/src/github.com/project-serum/anchor/target/doc/bytemuck/trait.Pod.html#safety)
/// section before using.
#[proc_macro_attribute]
pub fn account(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut namespace = "".to_string();
    let mut is_zero_copy = false;
    if args.to_string().split(',').count() > 2 {
        panic!("Only two args are allowed to the account attribute.")
    }
    for arg in args.to_string().split(',') {
        let ns = arg
            .to_string()
            .replace("\"", "")
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if ns == "zero_copy" {
            is_zero_copy = true;
        } else {
            namespace = ns;
        }
    }

    let account_strct = parse_macro_input!(input as syn::ItemStruct);
    let account_name = &account_strct.ident;
    let (impl_gen, type_gen, where_clause) = account_strct.generics.split_for_impl();

    let discriminator: proc_macro2::TokenStream = {
        // Namespace the discriminator to prevent collisions.
        let discriminator_preimage = {
            // For now, zero copy accounts can't be namespaced.
            if namespace.is_empty() {
                format!("account:{}", account_name.to_string())
            } else {
                format!("{}:{}", namespace, account_name.to_string())
            }
        };

        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(
            &anchor_syn::hash::hash(discriminator_preimage.as_bytes()).to_bytes()[..8],
        );
        format!("{:?}", discriminator).parse().unwrap()
    };

    proc_macro::TokenStream::from({
        if is_zero_copy {
            quote! {
                #[zero_copy]
                #account_strct

                #[automatically_derived]
                unsafe impl #impl_gen anchor_lang::__private::bytemuck::Pod for #account_name #type_gen #where_clause {}
                #[automatically_derived]
                unsafe impl #impl_gen anchor_lang::__private::bytemuck::Zeroable for #account_name #type_gen #where_clause {}

                #[automatically_derived]
                impl #impl_gen anchor_lang::ZeroCopy for #account_name #type_gen #where_clause {}

                #[automatically_derived]
                impl #impl_gen anchor_lang::Discriminator for #account_name #type_gen #where_clause {
                    fn discriminator() -> [u8; 8] {
                        #discriminator
                    }
                }

                // This trait is useful for clients deserializing accounts.
                // It's expected on-chain programs deserialize via zero-copy.
                #[automatically_derived]
                impl #impl_gen anchor_lang::AccountDeserialize for #account_name #type_gen #where_clause {
                    fn try_deserialize(buf: &mut &[u8]) -> std::result::Result<Self, ProgramError> {
                        if buf.len() < #discriminator.len() {
                            return Err(anchor_lang::__private::ErrorCode::AccountDiscriminatorNotFound.into());
                        }
                        let given_disc = &buf[..8];
                        if &#discriminator != given_disc {
                            return Err(anchor_lang::__private::ErrorCode::AccountDiscriminatorMismatch.into());
                        }
                        Self::try_deserialize_unchecked(buf)
                    }

                    fn try_deserialize_unchecked(buf: &mut &[u8]) -> std::result::Result<Self, ProgramError> {
                        let data: &[u8] = &buf[8..];
                        // Re-interpret raw bytes into the POD data structure.
                        let account = anchor_lang::__private::bytemuck::from_bytes(data);
                        // Copy out the bytes into a new, owned data structure.
                        Ok(*account)
                    }
                }
            }
        } else {
            quote! {
                #[derive(AnchorSerialize, AnchorDeserialize, Clone)]
                #account_strct

                #[automatically_derived]
                impl #impl_gen anchor_lang::AccountSerialize for #account_name #type_gen #where_clause {
                    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> std::result::Result<(), ProgramError> {
                        writer.write_all(&#discriminator).map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotSerialize)?;
                        AnchorSerialize::serialize(
                            self,
                            writer
                        )
                            .map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotSerialize)?;
                        Ok(())
                    }
                }

                #[automatically_derived]
                impl #impl_gen anchor_lang::AccountDeserialize for #account_name #type_gen #where_clause {
                    fn try_deserialize(buf: &mut &[u8]) -> std::result::Result<Self, ProgramError> {
                        if buf.len() < #discriminator.len() {
                            return Err(anchor_lang::__private::ErrorCode::AccountDiscriminatorNotFound.into());
                        }
                        let given_disc = &buf[..8];
                        if &#discriminator != given_disc {
                            return Err(anchor_lang::__private::ErrorCode::AccountDiscriminatorMismatch.into());
                        }
                        Self::try_deserialize_unchecked(buf)
                    }

                    fn try_deserialize_unchecked(buf: &mut &[u8]) -> std::result::Result<Self, ProgramError> {
                        let mut data: &[u8] = &buf[8..];
                        AnchorDeserialize::deserialize(&mut data)
                            .map_err(|_| anchor_lang::__private::ErrorCode::AccountDidNotDeserialize.into())
                    }
                }

                #[automatically_derived]
                impl #impl_gen anchor_lang::Discriminator for #account_name #type_gen #where_clause {
                    fn discriminator() -> [u8; 8] {
                        #discriminator
                    }
                }
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
                        format!("get_{}", field_name.to_string()).parse().unwrap();
                    let set_field: proc_macro2::TokenStream =
                        format!("set_{}", field_name.to_string()).parse().unwrap();

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
/// This is just a convenient alias for
///
/// ```ignore
/// #[derive(Copy, Clone)]
/// #[repr(packed)]
/// struct MyStruct {...}
/// ```
#[proc_macro_attribute]
pub fn zero_copy(
    _args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let account_strct = parse_macro_input!(item as syn::ItemStruct);

    proc_macro::TokenStream::from(quote! {
        #[derive(anchor_lang::__private::ZeroCopyAccessor, Copy, Clone)]
        #[repr(packed)]
        #account_strct
    })
}
