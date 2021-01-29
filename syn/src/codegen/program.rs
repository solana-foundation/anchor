use crate::parser;
use crate::{Program, RpcArg, State};
use heck::CamelCase;
use quote::quote;

pub fn generate(program: Program) -> proc_macro2::TokenStream {
    let mod_name = &program.name;
    let instruction_name = instruction_enum_name(&program);
    let dispatch = generate_dispatch(&program);
    let handlers_non_inlined = generate_non_inlined_handlers(&program);
    let methods = generate_methods(&program);
    let instruction = generate_instruction(&program);
    let cpi = generate_cpi(&program);

    quote! {
        // Import everything in the mod, in case the user wants to put types
        // in there.
        use #mod_name::*;

        #[cfg(not(feature = "no-entrypoint"))]
        anchor_lang::solana_program::entrypoint!(entry);
        #[cfg(not(feature = "no-entrypoint"))]
        fn entry(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
            if cfg!(not(feature = "no-idl")) {
                if instruction_data.len() >= 8 {
                    if anchor_lang::idl::IDL_IX_TAG.to_le_bytes() == instruction_data[..8] {
                        return __private::__idl(program_id, accounts, &instruction_data[8..]);
                    }
                }
            }
            let mut data: &[u8] = instruction_data;
            let ix = __private::instruction::#instruction_name::deserialize(&mut data)
                .map_err(|_| ProgramError::Custom(1))?; // todo: error code

                #dispatch
        }

        // Create a private module to not clutter the program's namespace.
        mod __private {
            use super::*;

            #handlers_non_inlined

            #instruction
        }

        #methods

        #cpi
    }
}

pub fn generate_dispatch(program: &Program) -> proc_macro2::TokenStream {
    let ctor_state_dispatch_arm = match &program.state {
        None => quote! { /* no-op */ },
        Some(state) => {
            let variant_arm = generate_ctor_variant(program, state);
            let ctor_args = generate_ctor_args(state);
            quote! {
                __private::instruction::#variant_arm => __private::__ctor(program_id, accounts, #(#ctor_args),*),
            }
        }
    };
    let state_dispatch_arms: Vec<proc_macro2::TokenStream> = match &program.state {
        None => vec![],
        Some(s) => s
            .methods
            .iter()
            .map(|rpc: &crate::StateRpc| {
                let rpc_arg_names: Vec<&syn::Ident> =
                    rpc.args.iter().map(|arg| &arg.name).collect();
                let variant_arm: proc_macro2::TokenStream = generate_ix_variant(
                    program,
                    rpc.raw_method.sig.ident.to_string(),
                    &rpc.args,
                    true,
                );
                let rpc_name: proc_macro2::TokenStream = {
                    let name = &rpc.raw_method.sig.ident.to_string();
                    format!("__{}", name).parse().unwrap()
                };
                quote! {
                    __private::instruction::#variant_arm => {
                        __private::#rpc_name(program_id, accounts, #(#rpc_arg_names),*)
                    }
                }
            })
            .collect(),
    };
    let dispatch_arms: Vec<proc_macro2::TokenStream> = program
        .rpcs
        .iter()
        .map(|rpc| {
            let rpc_arg_names: Vec<&syn::Ident> = rpc.args.iter().map(|arg| &arg.name).collect();
            let variant_arm = generate_ix_variant(
                program,
                rpc.raw_method.sig.ident.to_string(),
                &rpc.args,
                false,
            );
            let rpc_name = &rpc.raw_method.sig.ident;
            quote! {
                __private::instruction::#variant_arm => {
                    __private::#rpc_name(program_id, accounts, #(#rpc_arg_names),*)
                }
            }
        })
        .collect();

    quote! {
        match ix {
            #ctor_state_dispatch_arm
            #(#state_dispatch_arms),*
            #(#dispatch_arms),*
        }
    }
}

// Generate non-inlined wrappers for each instruction handler, since Solana's
// BPF max stack size can't handle reasonable sized dispatch trees without doing
// so.
pub fn generate_non_inlined_handlers(program: &Program) -> proc_macro2::TokenStream {
    let program_name = &program.name;
    let non_inlined_idl: proc_macro2::TokenStream = {
        quote! {
            // Entry for all IDL related instructions. Use the "no-idl" feature
            // to eliminate this code, for example, if one wants to make the
            // IDL no longer mutable or if one doesn't want to store the IDL
            // on chain.
            #[inline(never)]
            #[cfg(not(feature = "no-idl"))]
            pub fn __idl(program_id: &Pubkey, accounts: &[AccountInfo], idl_ix_data: &[u8]) -> ProgramResult {
                let mut accounts = accounts;
                let mut data: &[u8] = idl_ix_data;

                let ix = anchor_lang::idl::IdlInstruction::deserialize(&mut data)
                    .map_err(|_| ProgramError::Custom(1))?; // todo

                match ix {
                    anchor_lang::idl::IdlInstruction::Create { data_len } => {
                        let mut accounts = anchor_lang::idl::IdlCreateAccounts::try_accounts(program_id, &mut accounts)?;
                        __idl_create_account(program_id, &mut accounts, data_len)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::Write { data } => {
                        let mut accounts = anchor_lang::idl::IdlAccounts::try_accounts(program_id, &mut accounts)?;
                        __idl_write(program_id, &mut accounts, data)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::Clear => {
                        let mut accounts = anchor_lang::idl::IdlAccounts::try_accounts(program_id, &mut accounts)?;
                        __idl_clear(program_id, &mut accounts)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::SetAuthority { new_authority } => {
                        let mut accounts = anchor_lang::idl::IdlAccounts::try_accounts(program_id, &mut accounts)?;
                        __idl_set_authority(program_id, &mut accounts, new_authority)?;
                        accounts.exit(program_id)?;
                    }
                }
                Ok(())
            }

            // One time IDL account initializer. Will faill on subsequent
            // invocations.
            #[inline(never)]
            pub fn __idl_create_account(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlCreateAccounts,
                data_len: u64,
            ) -> ProgramResult {
                // Create the IDL's account.
                let from = accounts.from.key;
                let (base, nonce) = Pubkey::find_program_address(&[], accounts.program.key);
                let seed = anchor_lang::idl::IdlAccount::seed();
                let owner = accounts.program.key;
                let to = Pubkey::create_with_seed(&base, seed, owner).unwrap();
                // Space: account discriminator || authority pubkey || vec len || vec data
                let space = 8 + 32 + 4 + data_len as usize;
                let lamports = accounts.rent.minimum_balance(space);
                let seeds = &[&[nonce][..]];
                let ix = anchor_lang::solana_program::system_instruction::create_account_with_seed(
                    from,
                    &to,
                    &base,
                    seed,
                    lamports,
                    space as u64,
                    owner,
                );
                anchor_lang::solana_program::program::invoke_signed(
                    &ix,
                    &[
                        accounts.from.clone(),
                        accounts.to.clone(),
                        accounts.base.clone(),
                        accounts.system_program.clone(),
                    ],
                    &[seeds],
                )?;

                // Deserialize the newly created account.
                let mut idl_account = {
                    let mut account_data =  accounts.to.try_borrow_data()?;
                    let mut account_data_slice: &[u8] = &account_data;
                    anchor_lang::idl::IdlAccount::try_deserialize_unchecked(
                        &mut account_data_slice,
                    )?
                };

                // Set the authority.
                idl_account.authority = *accounts.from.key;

                // Store the new account data.
                let mut data = accounts.to.try_borrow_mut_data()?;
                let dst: &mut [u8] = &mut data;
                let mut cursor = std::io::Cursor::new(dst);
                idl_account.try_serialize(&mut cursor)?;

                Ok(())
            }

            #[inline(never)]
            pub fn __idl_write(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlAccounts,
                idl_data: Vec<u8>,
            ) -> ProgramResult {
                let mut idl = &mut accounts.idl;
                idl.data.extend(idl_data);
                Ok(())
            }

            #[inline(never)]
            pub fn __idl_clear(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlAccounts,
            ) -> ProgramResult {
                accounts.idl.data = vec![];
                Ok(())
            }

            #[inline(never)]
            pub fn __idl_set_authority(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlAccounts,
                new_authority: Pubkey,
            ) -> ProgramResult {
                accounts.idl.authority = new_authority;
                Ok(())
            }
        }
    };
    let non_inlined_ctor: proc_macro2::TokenStream = match &program.state {
        None => quote! {},
        Some(state) => {
            let ctor_typed_args = generate_ctor_typed_args(state);
            let ctor_untyped_args = generate_ctor_args(state);
            let name = &state.strct.ident;
            let mod_name = &program.name;
            let anchor_ident = &state.ctor_anchor;
            quote! {
                // One time state account initializer. Will faill on subsequent
                // invocations.
                #[inline(never)]
                pub fn __ctor(program_id: &Pubkey, accounts: &[AccountInfo], #(#ctor_typed_args),*) -> ProgramResult {
                    let mut remaining_accounts: &[AccountInfo] = accounts;

                    // Deserialize accounts.
                    let ctor_accounts = anchor_lang::Ctor::try_accounts(program_id, &mut remaining_accounts)?;
                    let mut ctor_user_def_accounts = #anchor_ident::try_accounts(program_id, &mut remaining_accounts)?;

                    // Invoke the ctor.
                    let instance = #mod_name::#name::new(
                        anchor_lang::Context::new(
                            program_id,
                            &mut ctor_user_def_accounts,
                            remaining_accounts,
                        ),
                        #(#ctor_untyped_args),*
                    )?;

                    // Create the solana account for the ctor data.
                    let from = ctor_accounts.from.key;
                    let (base, nonce) = Pubkey::find_program_address(&[], ctor_accounts.program.key);
                    let seed = anchor_lang::ProgramState::<#name>::seed();
                    let owner = ctor_accounts.program.key;
                    let to = Pubkey::create_with_seed(&base, seed, owner).unwrap();
                    // Add 8 for the account discriminator.
                    let space = 8 + instance.try_to_vec().map_err(|_| ProgramError::Custom(1))?.len();
                    let lamports = ctor_accounts.rent.minimum_balance(space);
                    let seeds = &[&[nonce][..]];
                    let ix = anchor_lang::solana_program::system_instruction::create_account_with_seed(
                        from,
                        &to,
                        &base,
                        seed,
                        lamports,
                        space as u64,
                        owner,
                    );
                    anchor_lang::solana_program::program::invoke_signed(
                        &ix,
                        &[
                            ctor_accounts.from.clone(),
                            ctor_accounts.to.clone(),
                            ctor_accounts.base.clone(),
                            ctor_accounts.system_program.clone(),
                        ],
                        &[seeds],
                    )?;

                    // Serialize the state and save it to storage.
                    ctor_user_def_accounts.exit(program_id)?;
                    let mut data = ctor_accounts.to.try_borrow_mut_data()?;
                    let dst: &mut [u8] = &mut data;
                    let mut cursor = std::io::Cursor::new(dst);
                    instance.try_serialize(&mut cursor)?;

                    Ok(())
                }
            }
        }
    };
    let non_inlined_state_handlers: Vec<proc_macro2::TokenStream> = match &program.state {
        None => vec![],
        Some(state) => state
            .methods
            .iter()
            .map(|rpc| {
                let rpc_params: Vec<_> = rpc.args.iter().map(|arg| &arg.raw_arg).collect();
                let rpc_arg_names: Vec<&syn::Ident> =
                    rpc.args.iter().map(|arg| &arg.name).collect();
                let private_rpc_name: proc_macro2::TokenStream = {
                    let n = format!("__{}", &rpc.raw_method.sig.ident.to_string());
                    n.parse().unwrap()
                };
                let rpc_name = &rpc.raw_method.sig.ident;
                let state_ty: proc_macro2::TokenStream = state.name.parse().unwrap();
                let anchor_ident = &rpc.anchor_ident;
                quote! {
                    #[inline(never)]
                    pub fn #private_rpc_name(
                        program_id: &Pubkey,
                        accounts: &[AccountInfo],
                        #(#rpc_params),*
                    ) -> ProgramResult {

                        let mut remaining_accounts: &[AccountInfo] = accounts;
                        if remaining_accounts.len() == 0 {
                            return Err(ProgramError::Custom(1)); // todo
                        }

                        // Deserialize the program state account.
                        let state_account = &remaining_accounts[0];
                        let mut state: #state_ty = {
                            let data = state_account.try_borrow_data()?;
                            let mut sliced: &[u8] = &data;
                            anchor_lang::AccountDeserialize::try_deserialize(&mut sliced)?
                        };

                        remaining_accounts = &remaining_accounts[1..];

                        // Deserialize the program's execution context.
                        let mut accounts = #anchor_ident::try_accounts(
                            program_id,
                            &mut remaining_accounts,
                        )?;
                        let ctx = Context::new(program_id, &mut accounts, remaining_accounts);

                        // Execute user defined function.
                        state.#rpc_name(
                            ctx,
                            #(#rpc_arg_names),*
                        )?;

                        // Serialize the state and save it to storage.
                        accounts.exit(program_id)?;
                        let mut data = state_account.try_borrow_mut_data()?;
                        let dst: &mut [u8] = &mut data;
                        let mut cursor = std::io::Cursor::new(dst);
                        state.try_serialize(&mut cursor)?;

                        Ok(())
                    }
                }
            })
            .collect(),
    };
    let non_inlined_handlers: Vec<proc_macro2::TokenStream> = program
        .rpcs
        .iter()
        .map(|rpc| {
            let rpc_params: Vec<_> = rpc.args.iter().map(|arg| &arg.raw_arg).collect();
            let rpc_arg_names: Vec<&syn::Ident> = rpc.args.iter().map(|arg| &arg.name).collect();
            let rpc_name = &rpc.raw_method.sig.ident;
            let anchor = &rpc.anchor_ident;

            quote! {
                #[inline(never)]
                pub fn #rpc_name(
                    program_id: &Pubkey,
                    accounts: &[AccountInfo],
                    #(#rpc_params),*
                ) -> ProgramResult {
                    let mut remaining_accounts: &[AccountInfo] = accounts;
                    let mut accounts = #anchor::try_accounts(program_id, &mut remaining_accounts)?;
                    #program_name::#rpc_name(
                        Context::new(program_id, &mut accounts, remaining_accounts),
                        #(#rpc_arg_names),*
                    )?;
                    accounts.exit(program_id)
                }
            }
        })
        .collect();

    quote! {
        #non_inlined_idl
        #non_inlined_ctor
        #(#non_inlined_state_handlers)*
        #(#non_inlined_handlers)*
    }
}

pub fn generate_ctor_variant(program: &Program, state: &State) -> proc_macro2::TokenStream {
    let enum_name = instruction_enum_name(program);
    let ctor_args = generate_ctor_args(state);
    if ctor_args.len() == 0 {
        quote! {
            #enum_name::__Ctor
        }
    } else {
        quote! {
            #enum_name::__Ctor {
                #(#ctor_args),*
            }
        }
    }
}

pub fn generate_ctor_typed_variant_with_comma(program: &Program) -> proc_macro2::TokenStream {
    match &program.state {
        None => quote! {},
        Some(state) => {
            let ctor_args = generate_ctor_typed_args(state);
            if ctor_args.len() == 0 {
                quote! {
                    __Ctor,
                }
            } else {
                quote! {
                    __Ctor {
                        #(#ctor_args),*
                    },
                }
            }
        }
    }
}

fn generate_ctor_typed_args(state: &State) -> Vec<syn::PatType> {
    state
        .ctor
        .sig
        .inputs
        .iter()
        .filter_map(|arg: &syn::FnArg| match arg {
            syn::FnArg::Typed(pat_ty) => {
                let mut arg_str = parser::tts_to_string(&pat_ty.ty);
                arg_str.retain(|c| !c.is_whitespace());
                if arg_str.starts_with("Context<") {
                    return None;
                }
                Some(pat_ty.clone())
            }
            _ => panic!("Invalid syntaxe,"),
        })
        .collect()
}

fn generate_ctor_args(state: &State) -> Vec<Box<syn::Pat>> {
    state
        .ctor
        .sig
        .inputs
        .iter()
        .filter_map(|arg: &syn::FnArg| match arg {
            syn::FnArg::Typed(pat_ty) => {
                let mut arg_str = parser::tts_to_string(&pat_ty.ty);
                arg_str.retain(|c| !c.is_whitespace());
                if arg_str.starts_with("Context<") {
                    return None;
                }
                Some(pat_ty.pat.clone())
            }
            _ => panic!(""),
        })
        .collect()
}

pub fn generate_ix_variant(
    program: &Program,
    name: String,
    args: &[RpcArg],
    underscore: bool,
) -> proc_macro2::TokenStream {
    let enum_name = instruction_enum_name(program);
    let rpc_arg_names: Vec<&syn::Ident> = args.iter().map(|arg| &arg.name).collect();
    let rpc_name_camel: proc_macro2::TokenStream = {
        let n = name.to_camel_case();
        if underscore {
            format!("__{}", n).parse().unwrap()
        } else {
            n.parse().unwrap()
        }
    };

    if args.len() == 0 {
        quote! {
            #enum_name::#rpc_name_camel
        }
    } else {
        quote! {
            #enum_name::#rpc_name_camel {
                #(#rpc_arg_names),*
            }
        }
    }
}

pub fn generate_methods(program: &Program) -> proc_macro2::TokenStream {
    let program_mod = &program.program_mod;
    quote! {
        #program_mod
    }
}

pub fn generate_instruction(program: &Program) -> proc_macro2::TokenStream {
    let enum_name = instruction_enum_name(program);
    let ctor_variant = generate_ctor_typed_variant_with_comma(program);
    let state_method_variants: Vec<proc_macro2::TokenStream> = match &program.state {
        None => vec![],
        Some(state) => state
            .methods
            .iter()
            .map(|method| {
                let rpc_name_camel: proc_macro2::TokenStream = {
                    let name = format!(
                        "__{}",
                        &method.raw_method.sig.ident.to_string().to_camel_case(),
                    );
                    name.parse().unwrap()
                };
                let raw_args: Vec<&syn::PatType> =
                    method.args.iter().map(|arg| &arg.raw_arg).collect();
                // If no args, output a "unit" variant instead of a struct variant.
                if method.args.len() == 0 {
                    quote! {
                        #rpc_name_camel,
                    }
                } else {
                    quote! {
                        #rpc_name_camel {
                            #(#raw_args),*
                        },
                    }
                }
            })
            .collect(),
    };
    let variants: Vec<proc_macro2::TokenStream> = program
        .rpcs
        .iter()
        .map(|rpc| {
            let rpc_name_camel = proc_macro2::Ident::new(
                &rpc.raw_method.sig.ident.to_string().to_camel_case(),
                rpc.raw_method.sig.ident.span(),
            );
            let raw_args: Vec<&syn::PatType> = rpc.args.iter().map(|arg| &arg.raw_arg).collect();
            // If no args, output a "unit" variant instead of a struct variant.
            if rpc.args.len() == 0 {
                quote! {
                    #rpc_name_camel
                }
            } else {
                quote! {
                    #rpc_name_camel {
                        #(#raw_args),*
                    }
                }
            }
        })
        .collect();

    quote! {
        pub mod instruction {
            use super::*;
            #[derive(AnchorSerialize, AnchorDeserialize)]
            pub enum #enum_name {
                #ctor_variant
                #(#state_method_variants)*
                #(#variants),*
            }
        }
    }
}

fn instruction_enum_name(program: &Program) -> proc_macro2::Ident {
    proc_macro2::Ident::new(
        &format!("{}Instruction", program.name.to_string().to_camel_case()),
        program.name.span(),
    )
}

fn generate_cpi(program: &Program) -> proc_macro2::TokenStream {
    let cpi_methods: Vec<proc_macro2::TokenStream> = program
        .rpcs
        .iter()
        .map(|rpc| {
            let accounts_ident = &rpc.anchor_ident;
            let cpi_method = {
                let ix_variant = generate_ix_variant(
                    program,
                    rpc.raw_method.sig.ident.to_string(),
                    &rpc.args,
                    false,
                );
                let method_name = &rpc.ident;
                let args: Vec<&syn::PatType> = rpc.args.iter().map(|arg| &arg.raw_arg).collect();
                quote! {
                    pub fn #method_name<'a, 'b, 'c, 'info>(
                        ctx: CpiContext<'a, 'b, 'c, 'info, #accounts_ident<'info>>,
                        #(#args),*
                    ) -> ProgramResult {
                        let ix = {
                            let ix = __private::instruction::#ix_variant;
                            let data = AnchorSerialize::try_to_vec(&ix)
                                .map_err(|_| ProgramError::InvalidInstructionData)?;
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
            };

            cpi_method
        })
        .collect();
    quote! {
        #[cfg(feature = "cpi")]
        pub mod cpi {
            use super::*;

            #(#cpi_methods)*
        }
    }
}
