extern crate proc_macro;

use anchor_syn::parser;
use heck::SnakeCase;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn interface(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item_trait = parse_macro_input!(input as syn::ItemTrait);

    let trait_name = item_trait.ident.to_string();
    let mod_name: proc_macro2::TokenStream = item_trait
        .ident
        .to_string()
        .to_snake_case()
        .parse()
        .unwrap();

    let methods: Vec<proc_macro2::TokenStream> = item_trait
        .items
        .iter()
        .filter_map(|trait_item: &syn::TraitItem| match trait_item {
            syn::TraitItem::Method(m) => Some(m),
            _ => None,
        })
        .map(|method: &syn::TraitItemMethod| {
            let method_name = &method.sig.ident;
            let args: Vec<&syn::PatType> = method
                .sig
                .inputs
                .iter()
                .filter_map(|arg: &syn::FnArg| match arg {
                    syn::FnArg::Typed(pat_ty) => Some(pat_ty),
                    // TODO: just map this to None once we allow this feature.
                    _ => panic!("Invalid syntax. No self allowed."),
                })
                .filter_map(|pat_ty: &syn::PatType| {
                    let mut ty = parser::tts_to_string(&pat_ty.ty);
                    ty.retain(|s| !s.is_whitespace());
                    if ty.starts_with("Context<") {
                        None
                    } else {
                        Some(pat_ty)
                    }
                })
                .collect();
            let args_no_tys: Vec<&Box<syn::Pat>> = args
                .iter()
                .map(|arg| {
                    &arg.pat
                })
                .collect();
            let args_struct = {
                if args.len() == 0 {
                    quote! {
                        use anchor_lang::prelude::borsh;
                        #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
                        struct Args;
                    }
                } else {
                    quote! {
                        use anchor_lang::prelude::borsh;
                        #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
                        struct Args {
                            #(#args),*
                        }
                    }
                }
            };

            let sighash_arr = anchor_syn::codegen::program::sighash(&trait_name, &method_name.to_string());
            let sighash_tts: proc_macro2::TokenStream =
                format!("{:?}", sighash_arr).parse().unwrap();
            quote! {
                pub fn #method_name<'a,'b, 'c, 'info, T: anchor_lang::ToAccountMetas + anchor_lang::ToAccountInfos<'info>>(
                    ctx: anchor_lang::CpiContext<'a, 'b, 'c, 'info, T>,
                    #(#args),*
                ) -> anchor_lang::solana_program::entrypoint::ProgramResult {
                    #args_struct

                    let ix = {
                        let ix = Args {
                            #(#args_no_tys),*
                        };
                        let mut ix_data = anchor_lang::AnchorSerialize::try_to_vec(&ix)
                            .map_err(|_| anchor_lang::solana_program::program_error::ProgramError::InvalidInstructionData)?;
                        let mut data = #sighash_tts.to_vec();
                        data.append(&mut ix_data);
                        let accounts = ctx.accounts.to_account_metas(None);
                        anchor_lang::solana_program::instruction::Instruction {
                            program_id: *ctx.program.key,
                            accounts,
                            data,
                        }
                    };
                    let mut acc_infos = ctx.accounts.to_account_infos();
                    acc_infos.push(ctx.program.clone());
                    anchor_lang::solana_program::program::invoke_signed(
                        &ix,
                        &acc_infos,
                        ctx.signer_seeds,
                    )
                }
            }
        })
        .collect();

    proc_macro::TokenStream::from(quote! {
        #item_trait

        mod #mod_name {
            use super::*;
            #(#methods)*
        }
    })
}
