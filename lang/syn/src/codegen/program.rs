use crate::parser;
use crate::{IxArg, Program, State, StateIx};
use heck::{CamelCase, SnakeCase};
use quote::quote;

// Namespace for calculating state instruction sighash signatures.
const SIGHASH_STATE_NAMESPACE: &str = "state";

// Namespace for calculating instruction sighash signatures for any instruction
// not affecting program state.
const SIGHASH_GLOBAL_NAMESPACE: &str = "global";

pub fn generate(program: Program) -> proc_macro2::TokenStream {
    let mod_name = &program.name;
    let dispatch = generate_dispatch(&program);
    let handlers_non_inlined = generate_non_inlined_handlers(&program);
    let methods = generate_methods(&program);
    let ixs = generate_ixs(&program);
    let cpi = generate_cpi(&program);
    let accounts = generate_accounts(&program);

    quote! {
        // TODO: remove once we allow segmented paths in `Accounts` structs.
        use #mod_name::*;

        #[cfg(not(feature = "no-entrypoint"))]
        anchor_lang::solana_program::entrypoint!(entry);
        /// The Anchor codegen exposes a programming model where a user defines
        /// a set of methods inside of a `#[program]` module in a way similar
        /// to writing RPC request handlers. The macro then generates a bunch of
        /// code wrapping these user defined methods into something that can be
        /// executed on Solana.
        ///
        /// These methods fall into one of three categories, each of which
        /// can be considered a different "namespace" of the program.
        ///
        /// 1) Global methods - regular methods inside of the `#[program]`.
        /// 2) State methods - associated methods inside a `#[state]` struct.
        /// 3) Interface methods - methods inside a strait struct's
        ///    implementation of an `#[interface]` trait.
        ///
        /// Care must be taken by the codegen to prevent collisions between
        /// methods in these different namespaces. For this reason, Anchor uses
        /// a variant of sighash to perform method dispatch, rather than
        /// something like a simple enum variant discriminator.
        ///
        /// The execution flow of the generated code can be roughly outlined:
        ///
        /// * Start program via the entrypoint.
        /// * Strip method identifier off the first 8 bytes of the instruction
        ///   data and invoke the identified method. The method identifier
        ///   is a variant of sighash. See docs.rs for `anchor_lang` for details.
        /// * If the method identifier is an IDL identifier, execute the IDL
        ///   instructions, which are a special set of hardcoded instructions
        ///   baked into every Anchor program. Then exit.
        /// * Otherwise, the method identifier is for a user defined
        ///   instruction, i.e., one of the methods in the user defined
        ///   `#[program]` module. Perform method dispatch, i.e., execute the
        ///   big match statement mapping method identifier to method handler
        ///   wrapper.
        /// * Run the method handler wrapper. This wraps the code the user
        ///   actually wrote, deserializing the accounts, constructing the
        ///   context, invoking the user's code, and finally running the exit
        ///   routine, which typically persists account changes.
        ///
        /// The `entry` function here, defines the standard entry to a Solana
        /// program, where execution begins.
        #[cfg(not(feature = "no-entrypoint"))]
        fn entry(program_id: &Pubkey, accounts: &[AccountInfo], ix_data: &[u8]) -> ProgramResult {
            if ix_data.len() < 8 {
                return Err(ProgramError::Custom(99));
            }

            // Split the instruction data into the first 8 byte method
            // identifier (sighash) and the serialized instruction data.
            let mut ix_data: &[u8] = ix_data;
            let sighash: [u8; 8] = {
                let mut sighash: [u8; 8] = [0; 8];
                sighash.copy_from_slice(&ix_data[..8]);
                ix_data = &ix_data[8..];
                sighash
            };

            dispatch(program_id, accounts, sighash, ix_data)
                .map_err(|e| {
                    anchor_lang::solana_program::msg!(&e.to_string());
                    e
                })
        }

        #dispatch

        /// Create a private module to not clutter the program's namespace.
        /// Defines an entrypoint for each individual instruction handler
        /// wrapper.
        mod __private {
            use super::*;

            #handlers_non_inlined
        }

        #accounts

        #ixs

        #methods

        #cpi
    }
}

pub fn generate_dispatch(program: &Program) -> proc_macro2::TokenStream {
    // Dispatch the state constructor.
    let ctor_state_dispatch_arm = match &program.state {
        None => quote! { /* no-op */ },
        Some(state) => match state.ctor_and_anchor.is_some() {
            false => quote! {},
            true => {
                let variant_arm = generate_ctor_variant(state);
                let ctor_args = generate_ctor_args(state);
                let ix_name: proc_macro2::TokenStream =
                    generate_ctor_variant_name().parse().unwrap();
                let sighash_arr = sighash_ctor();
                let sighash_tts: proc_macro2::TokenStream =
                    format!("{:?}", sighash_arr).parse().unwrap();
                quote! {
                    #sighash_tts => {
                        let ix = instruction::state::#ix_name::deserialize(&mut ix_data)
                            .map_err(|_| ProgramError::Custom(1))?; // todo: error code
                        let instruction::state::#variant_arm = ix;
                        __private::__state::__ctor(program_id, accounts, #(#ctor_args),*)
                    }
                }
            }
        },
    };

    // Dispatch the state impl instructions.
    let state_dispatch_arms: Vec<proc_macro2::TokenStream> = match &program.state {
        None => vec![],
        Some(s) => s
            .impl_block_and_methods
            .as_ref()
            .map(|(_impl_block, methods)| {
                methods
                    .iter()
                    .map(|ix: &crate::StateIx| {
                        let ix_arg_names: Vec<&syn::Ident> =
                            ix.args.iter().map(|arg| &arg.name).collect();
                        let name = &ix.raw_method.sig.ident.to_string();
                        let ix_method_name: proc_macro2::TokenStream =
                        { format!("__{}", name).parse().unwrap() };
                        let variant_arm =
                            generate_ix_variant(ix.raw_method.sig.ident.to_string(), &ix.args);
                        let ix_name = generate_ix_variant_name(ix.raw_method.sig.ident.to_string());
                        let sighash_arr = sighash(SIGHASH_STATE_NAMESPACE, &name);
                        let sighash_tts: proc_macro2::TokenStream =
                            format!("{:?}", sighash_arr).parse().unwrap();
                        quote! {
                            #sighash_tts => {
                                let ix = instruction::state::#ix_name::deserialize(&mut ix_data)
                                    .map_err(|_| ProgramError::Custom(1))?; // todo: error code
                                let instruction::state::#variant_arm = ix;
                                __private::__state::#ix_method_name(program_id, accounts, #(#ix_arg_names),*)
                            }
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };

    // Dispatch all trait interface implementations.
    let trait_dispatch_arms: Vec<proc_macro2::TokenStream> = match &program.state {
        None => vec![],
        Some(s) => s
            .interfaces
            .as_ref()
            .map(|interfaces| {
                interfaces
                    .iter()
                    .flat_map(|iface: &crate::StateInterface| {
                        iface
                            .methods
                            .iter()
                            .map(|m: &crate::StateIx| {
                                let ix_arg_names: Vec<&syn::Ident> =
                                    m.args.iter().map(|arg| &arg.name).collect();
                                let name = &m.raw_method.sig.ident.to_string();
                                let ix_name: proc_macro2::TokenStream =  format!("__{}_{}", iface.trait_name, name).parse().unwrap();
                                let raw_args: Vec<&syn::PatType> = m
                                    .args
                                    .iter()
                                    .map(|arg: &crate::IxArg| &arg.raw_arg)
                                    .collect();
                                let sighash_arr = sighash(&iface.trait_name, &m.ident.to_string());
                                let sighash_tts: proc_macro2::TokenStream =
                                    format!("{:?}", sighash_arr).parse().unwrap();
                                let args_struct = {
                                    if m.args.is_empty() {
                                        quote! {
                                            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
                                            struct Args;
                                        }
                                    } else {
                                        quote! {
                                            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
                                            struct Args {
                                                #(#raw_args),*
                                            }
                                        }
                                    }
                                };
                                quote! {
                                    #sighash_tts => {
                                        #args_struct
                                        let ix = Args::deserialize(&mut ix_data)
                                            .map_err(|_| ProgramError::Custom(1))?; // todo: error code
                                        let Args {
                                            #(#ix_arg_names),*
                                        } = ix;
                                        __private::__interface::#ix_name(program_id, accounts, #(#ix_arg_names),*)
                                    }
                                }
                            })
                            .collect::<Vec<proc_macro2::TokenStream>>()
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    // Dispatch all global instructions.
    let global_dispatch_arms: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let ix_arg_names: Vec<&syn::Ident> = ix.args.iter().map(|arg| &arg.name).collect();
            let ix_method_name = &ix.raw_method.sig.ident;
            let ix_name = generate_ix_variant_name(ix.raw_method.sig.ident.to_string());
            let sighash_arr = sighash(SIGHASH_GLOBAL_NAMESPACE, &ix_method_name.to_string());
            let sighash_tts: proc_macro2::TokenStream =
                format!("{:?}", sighash_arr).parse().unwrap();
            let variant_arm = generate_ix_variant(ix.raw_method.sig.ident.to_string(), &ix.args);
            quote! {
                #sighash_tts => {
                    let ix = instruction::#ix_name::deserialize(&mut ix_data)
                        .map_err(|_| ProgramError::Custom(1))?; // todo: error code
                    let instruction::#variant_arm = ix;
                    __private::__global::#ix_method_name(program_id, accounts, #(#ix_arg_names),*)
                }
            }
        })
        .collect();

    quote! {
        /// Performs method dispatch.
        ///
        /// Each method in an anchor program is uniquely defined by a namespace
        /// and a rust identifier (i.e., the name given to the method). These
        /// two pieces can be combined to creater a method identifier,
        /// specifically, Anchor uses
        ///
        /// Sha256("<namespace>::<rust-identifier>")[..8],
        ///
        /// where the namespace can be one of three types. 1) "global" for a
        /// regular instruction, 2) "state" for a state struct instruction
        /// handler and 3) a trait namespace (used in combination with the
        /// `#[interface]` attribute), which is defined by the trait name, e..
        /// `MyTrait`.
        ///
        /// With this 8 byte identifier, Anchor performs method dispatch,
        /// matching the given 8 byte identifier to the associated method
        /// handler, which leads to user defined code being eventually invoked.
        fn dispatch(program_id: &Pubkey, accounts: &[AccountInfo], sighash: [u8; 8], mut ix_data: &[u8]) -> ProgramResult {
            // If the method identifier is the IDL tag, then execute an IDL
            // instruction, injected into all Anchor programs.
            if cfg!(not(feature = "no-idl")) {
                if sighash == anchor_lang::idl::IDL_IX_TAG.to_le_bytes() {
                    return __private::__idl::__idl_dispatch(program_id, accounts, &ix_data);
                }
            }

            match sighash {
                #ctor_state_dispatch_arm
                #(#state_dispatch_arms)*
                #(#trait_dispatch_arms)*
                #(#global_dispatch_arms)*
                _ => {
                    msg!("Fallback functions are not supported. If you have a use case, please file an issue.");
                    Err(ProgramError::Custom(99))
                }
            }
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
            pub fn __idl_dispatch(program_id: &Pubkey, accounts: &[AccountInfo], idl_ix_data: &[u8]) -> ProgramResult {
                let mut accounts = accounts;
                let mut data: &[u8] = idl_ix_data;

                let ix = anchor_lang::idl::IdlInstruction::deserialize(&mut data)
                    .map_err(|_| ProgramError::Custom(2))?; // todo

                match ix {
                    anchor_lang::idl::IdlInstruction::Create { data_len } => {
                        let mut accounts = anchor_lang::idl::IdlCreateAccounts::try_accounts(program_id, &mut accounts)?;
                        __idl_create_account(program_id, &mut accounts, data_len)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::CreateBuffer => {
                        let mut accounts = anchor_lang::idl::IdlCreateBuffer::try_accounts(program_id, &mut accounts)?;
                        __idl_create_buffer(program_id, &mut accounts)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::Write { data } => {
                        let mut accounts = anchor_lang::idl::IdlAccounts::try_accounts(program_id, &mut accounts)?;
                        __idl_write(program_id, &mut accounts, data)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::SetAuthority { new_authority } => {
                        let mut accounts = anchor_lang::idl::IdlAccounts::try_accounts(program_id, &mut accounts)?;
                        __idl_set_authority(program_id, &mut accounts, new_authority)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::SetBuffer => {
                        let mut accounts = anchor_lang::idl::IdlSetBuffer::try_accounts(program_id, &mut accounts)?;
                        __idl_set_buffer(program_id, &mut accounts)?;
                        accounts.exit(program_id)?;
                    },
                }
                Ok(())
            }

            #[inline(never)]
            #[cfg(feature = "no-idl")]
            pub fn __idl_dispatch(program_id: &Pubkey, accounts: &[AccountInfo], idl_ix_data: &[u8]) -> ProgramResult {
                Err(anchor_lang::solana_program::program_error::ProgramError::Custom(99))
            }

            // One time IDL account initializer. Will faill on subsequent
            // invocations.
            #[inline(never)]
            pub fn __idl_create_account(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlCreateAccounts,
                data_len: u64,
            ) -> ProgramResult {
                if program_id != accounts.program.key {
                    return Err(anchor_lang::solana_program::program_error::ProgramError::Custom(98)); // todo proper error
                }
                // Create the IDL's account.
                let from = accounts.from.key;
                let (base, nonce) = Pubkey::find_program_address(&[], program_id);
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
            pub fn __idl_create_buffer(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlCreateBuffer,
            ) -> ProgramResult {
                let mut buffer = &mut accounts.buffer;
                buffer.authority = *accounts.authority.key;
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
            pub fn __idl_set_authority(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlAccounts,
                new_authority: Pubkey,
            ) -> ProgramResult {
                accounts.idl.authority = new_authority;
                Ok(())
            }

            #[inline(never)]
            pub fn __idl_set_buffer(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlSetBuffer,
            ) -> ProgramResult {
                accounts.idl.data = accounts.buffer.data.clone();
                Ok(())
            }
        }
    };
    // Constructor handler.
    let non_inlined_ctor: proc_macro2::TokenStream = match &program.state {
        None => quote! {},
        Some(state) => match state.ctor_and_anchor.as_ref() {
            None => quote! {},
            Some((_ctor, anchor_ident)) => {
                let ctor_typed_args = generate_ctor_typed_args(state);
                let ctor_untyped_args = generate_ctor_args(state);
                let name = &state.strct.ident;
                let mod_name = &program.name;
                if state.is_zero_copy {
                    quote! {
                        // One time state account initializer. Will faill on subsequent
                        // invocations.
                        #[inline(never)]
                        pub fn __ctor(program_id: &Pubkey, accounts: &[AccountInfo], #(#ctor_typed_args),*) -> ProgramResult {
                            let mut remaining_accounts: &[AccountInfo] = accounts;

                            // Deserialize accounts.
                            let ctor_accounts = anchor_lang::__private::Ctor::try_accounts(program_id, &mut remaining_accounts)?;
                            let mut ctor_user_def_accounts = #anchor_ident::try_accounts(program_id, &mut remaining_accounts)?;

                            // Create the solana account for the ctor data.
                            let from = ctor_accounts.from.key;
                            let (base, nonce) = Pubkey::find_program_address(&[], ctor_accounts.program.key);
                            let seed = anchor_lang::__private::PROGRAM_STATE_SEED;
                            let owner = ctor_accounts.program.key;
                            let to = Pubkey::create_with_seed(&base, seed, owner).unwrap();
                            let space = 8 + std::mem::size_of::<#name>();
                            let lamports = ctor_accounts.rent.minimum_balance(std::convert::TryInto::try_into(space).unwrap());
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

                            // Zero copy deserialize.
                            let loader: anchor_lang::Loader<#mod_name::#name> = anchor_lang::Loader::try_from_init(&ctor_accounts.to)?;

                            // Invoke the ctor in a new lexical scope so that
                            // the zero-copy RefMut gets dropped. Required
                            // so that we can subsequently run the exit routine.
                            {
                                let mut instance = loader.load_init()?;
                                instance.new(
                                    anchor_lang::Context::new(
                                        program_id,
                                        &mut ctor_user_def_accounts,
                                        remaining_accounts,
                                    ),
                                    #(#ctor_untyped_args),*
                                )?;
                            }

                            // Exit routines.
                            ctor_user_def_accounts.exit(program_id)?;
                            loader.exit(program_id)?;

                            Ok(())
                        }
                    }
                } else {
                    quote! {
                        // One time state account initializer. Will faill on subsequent
                        // invocations.
                        #[inline(never)]
                        pub fn __ctor(program_id: &Pubkey, accounts: &[AccountInfo], #(#ctor_typed_args),*) -> ProgramResult {
                            let mut remaining_accounts: &[AccountInfo] = accounts;

                            // Deserialize accounts.
                            let ctor_accounts = anchor_lang::__private::Ctor::try_accounts(program_id, &mut remaining_accounts)?;
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
                            let space = anchor_lang::__private::AccountSize::size(&instance)?;
                            let lamports = ctor_accounts.rent.minimum_balance(std::convert::TryInto::try_into(space).unwrap());
                            let seeds = &[&[nonce][..]];
                            let ix = anchor_lang::solana_program::system_instruction::create_account_with_seed(
                                from,
                                &to,
                                &base,
                                seed,
                                lamports,
                                space,
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
            }
        },
    };

    // State method handlers.
    let non_inlined_state_handlers: Vec<proc_macro2::TokenStream> = match &program.state {
        None => vec![],
        Some(state) => state
            .impl_block_and_methods
            .as_ref()
            .map(|(_impl_block, methods)| {
                methods
                    .iter()
                    .map(|ix| {
                        let ix_params: Vec<_> = ix.args.iter().map(|arg| &arg.raw_arg).collect();
                        let ix_arg_names: Vec<&syn::Ident> =
                            ix.args.iter().map(|arg| &arg.name).collect();
                        let private_ix_name: proc_macro2::TokenStream = {
                            let n = format!("__{}", &ix.raw_method.sig.ident.to_string());
                            n.parse().unwrap()
                        };
                        let ix_name = &ix.raw_method.sig.ident;
                        let state_ty: proc_macro2::TokenStream = state.name.parse().unwrap();
                        let anchor_ident = &ix.anchor_ident;
                        let name = &state.strct.ident;
                        let mod_name = &program.name;

                        if state.is_zero_copy {
                            quote! {
                                #[inline(never)]
                                pub fn #private_ix_name(
                                    program_id: &Pubkey,
                                    accounts: &[AccountInfo],
                                    #(#ix_params),*
                                ) -> ProgramResult {
                                    let mut remaining_accounts: &[AccountInfo] = accounts;
                                    if remaining_accounts.is_empty() {
                                        return Err(ProgramError::Custom(1)); // todo
                                    }

                                    let state_account = &remaining_accounts[0];
                                    let loader: anchor_lang::Loader<#mod_name::#name> = anchor_lang::Loader::try_from(&state_account)?;
                                    remaining_accounts = &remaining_accounts[1..];

                                    // Deserialize the program's execution context.
                                    let mut accounts = #anchor_ident::try_accounts(
                                        program_id,
                                        &mut remaining_accounts,
                                    )?;
                                    let ctx = Context::new(program_id, &mut accounts, remaining_accounts);
                                    // Execute user defined function.
                                    {
                                        let mut state = loader.load_mut()?;
                                        state.#ix_name(
                                            ctx,
                                            #(#ix_arg_names),*
                                        )?;
                                    }
                                    // Serialize the state and save it to storage.
                                    accounts.exit(program_id)?;
                                    loader.exit(program_id)?;

                                    Ok(())
                                }
                            }
                        } else {
                            quote! {
                                #[inline(never)]
                                pub fn #private_ix_name(
                                    program_id: &Pubkey,
                                    accounts: &[AccountInfo],
                                    #(#ix_params),*
                                ) -> ProgramResult {
                                    let mut remaining_accounts: &[AccountInfo] = accounts;
                                    if remaining_accounts.is_empty() {
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
                                    state.#ix_name(
                                        ctx,
                                        #(#ix_arg_names),*
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
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };

    // State trait handlers.
    let non_inlined_state_trait_handlers: Vec<proc_macro2::TokenStream> = match &program.state {
        None => Vec::new(),
        Some(state) => state
            .interfaces
            .as_ref()
            .map(|interfaces| {
                interfaces
                    .iter()
                    .flat_map(|iface: &crate::StateInterface| {
                        iface
                            .methods
                            .iter()
                            .map(|ix| {
                                let ix_params: Vec<_> = ix.args.iter().map(|arg| &arg.raw_arg).collect();
                                let ix_arg_names: Vec<&syn::Ident> =
                                    ix.args.iter().map(|arg| &arg.name).collect();
                                let private_ix_name: proc_macro2::TokenStream = {
                                    let n = format!("__{}_{}", iface.trait_name, &ix.raw_method.sig.ident.to_string());
                                    n.parse().unwrap()
                                };
                                let ix_name = &ix.raw_method.sig.ident;
                                let state_ty: proc_macro2::TokenStream = state.name.parse().unwrap();
                                let anchor_ident = &ix.anchor_ident;

                                if state.is_zero_copy {
                                    // Easy to implement. Just need to write a test.
                                    // Feel free to open a PR.
                                    panic!("Trait implementations not yet implemented for zero copy state structs. Please file an issue.");
                                }

                                if ix.has_receiver {
                                    quote! {
                                        #[inline(never)]
                                        pub fn #private_ix_name(
                                            program_id: &Pubkey,
                                            accounts: &[AccountInfo],
                                            #(#ix_params),*
                                        ) -> ProgramResult {

                                            let mut remaining_accounts: &[AccountInfo] = accounts;
                                            if remaining_accounts.is_empty() {
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
                                            state.#ix_name(
                                                ctx,
                                                #(#ix_arg_names),*
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
                                } else {
                                    let state_name: proc_macro2::TokenStream = state.name.parse().unwrap();
                                    quote! {
                                        #[inline(never)]
                                        pub fn #private_ix_name(
                                            program_id: &Pubkey,
                                            accounts: &[AccountInfo],
                                            #(#ix_params),*
                                        ) -> ProgramResult {
                                            let mut remaining_accounts: &[AccountInfo] = accounts;
                                            let mut accounts = #anchor_ident::try_accounts(
                                                program_id,
                                                &mut remaining_accounts,
                                            )?;
                                            #state_name::#ix_name(
                                                Context::new(program_id, &mut accounts, remaining_accounts),
                                                #(#ix_arg_names),*
                                            )?;
                                            accounts.exit(program_id)
                                        }
                                    }
                                }
                            })
                            .collect::<Vec<proc_macro2::TokenStream>>()
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };
    let non_inlined_handlers: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let ix_params: Vec<_> = ix.args.iter().map(|arg| &arg.raw_arg).collect();
            let ix_arg_names: Vec<&syn::Ident> = ix.args.iter().map(|arg| &arg.name).collect();
            let ix_name = &ix.raw_method.sig.ident;
            let anchor = &ix.anchor_ident;

            quote! {
                #[inline(never)]
                pub fn #ix_name(
                    program_id: &Pubkey,
                    accounts: &[AccountInfo],
                    #(#ix_params),*
                ) -> ProgramResult {
                    let mut remaining_accounts: &[AccountInfo] = accounts;
                    let mut accounts = #anchor::try_accounts(program_id, &mut remaining_accounts)?;
                    #program_name::#ix_name(
                        Context::new(program_id, &mut accounts, remaining_accounts),
                        #(#ix_arg_names),*
                    )?;
                    accounts.exit(program_id)
                }
            }
        })
        .collect();

    quote! {
        /// __idl mod defines handlers for injected Anchor IDL instructions.
        pub mod __idl {
            use super::*;

            #non_inlined_idl
        }

        /// __state mod defines wrapped handlers for state instructions.
        pub mod __state {
            use super::*;

            #non_inlined_ctor
            #(#non_inlined_state_handlers)*
        }

        /// __interface mod defines wrapped handlers for `#[interface]` trait
        /// implementations.
        pub mod __interface {
            use super::*;

            #(#non_inlined_state_trait_handlers)*
        }

        /// __global mod defines wrapped handlers for global instructions.
        pub mod __global {
            use super::*;

            #(#non_inlined_handlers)*
        }
    }
}

pub fn generate_ctor_variant(state: &State) -> proc_macro2::TokenStream {
    let ctor_args = generate_ctor_args(state);
    let ctor_variant_name: proc_macro2::TokenStream = generate_ctor_variant_name().parse().unwrap();
    if ctor_args.is_empty() {
        quote! {
            #ctor_variant_name
        }
    } else {
        quote! {
            #ctor_variant_name {
                #(#ctor_args),*
            }
        }
    }
}

pub fn generate_ctor_variant_name() -> String {
    "Ctor".to_string()
}

fn generate_ctor_typed_args(state: &State) -> Vec<syn::PatType> {
    state
        .ctor_and_anchor
        .as_ref()
        .map(|(ctor, _anchor_ident)| {
            ctor.sig
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
                    _ => {
                        if !state.is_zero_copy {
                            panic!("Cannot pass self as parameter")
                        }
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn generate_ctor_args(state: &State) -> Vec<syn::Pat> {
    state
        .ctor_and_anchor
        .as_ref()
        .map(|(ctor, _anchor_ident)| {
            ctor.sig
                .inputs
                .iter()
                .filter_map(|arg: &syn::FnArg| match arg {
                    syn::FnArg::Typed(pat_ty) => {
                        let mut arg_str = parser::tts_to_string(&pat_ty.ty);
                        arg_str.retain(|c| !c.is_whitespace());
                        if arg_str.starts_with("Context<") {
                            return None;
                        }
                        Some(*pat_ty.pat.clone())
                    }
                    _ => {
                        if !state.is_zero_copy {
                            panic!("Cannot pass self as parameter");
                        }
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn generate_ix_variant(name: String, args: &[IxArg]) -> proc_macro2::TokenStream {
    let ix_arg_names: Vec<&syn::Ident> = args.iter().map(|arg| &arg.name).collect();
    let ix_name_camel: proc_macro2::TokenStream = {
        let n = name.to_camel_case();
        n.parse().unwrap()
    };

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

pub fn generate_ix_variant_name(name: String) -> proc_macro2::TokenStream {
    let n = name.to_camel_case();
    n.parse().unwrap()
}

pub fn generate_methods(program: &Program) -> proc_macro2::TokenStream {
    let program_mod = &program.program_mod;
    quote! {
        #program_mod
    }
}

pub fn generate_ixs(program: &Program) -> proc_macro2::TokenStream {
    let ctor_variant = match &program.state {
        None => quote! {},
        Some(state) => {
            let ctor_args: Vec<proc_macro2::TokenStream> = generate_ctor_typed_args(state)
                .iter()
                .map(|arg| {
                    format!("pub {}", parser::tts_to_string(&arg))
                        .parse()
                        .unwrap()
                })
                .collect();
            let strct = {
                if ctor_args.is_empty() {
                    quote! {
                        #[derive(AnchorSerialize, AnchorDeserialize)]
                        pub struct Ctor;
                    }
                } else {
                    quote! {
                        #[derive(AnchorSerialize, AnchorDeserialize)]
                        pub struct Ctor {
                            #(#ctor_args),*
                        }
                    }
                }
            };
            let sighash_arr = sighash_ctor();
            let sighash_tts: proc_macro2::TokenStream =
                format!("{:?}", sighash_arr).parse().unwrap();
            quote! {
                /// Instruction arguments to the `#[state]`'s `new`
                /// constructor.
                #strct

                impl anchor_lang::InstructionData for Ctor {
                    fn data(&self) -> Vec<u8> {
                        let mut d = #sighash_tts.to_vec();
                        d.append(&mut self.try_to_vec().expect("Should always serialize"));
                        d
                    }
                }
            }
        }
    };
    let state_method_variants: Vec<proc_macro2::TokenStream> = match &program.state {
        None => vec![],
        Some(state) => state
            .impl_block_and_methods
            .as_ref()
            .map(|(_impl_block, methods)| {
                methods
                    .iter()
                    .map(|method| {
                        let ix_name_camel: proc_macro2::TokenStream = method
                            .raw_method
                            .sig
                            .ident
                            .to_string()
                            .to_camel_case()
                            .parse()
                            .unwrap();
                        let raw_args: Vec<proc_macro2::TokenStream> = method
                            .args
                            .iter()
                            .map(|arg| {
                                format!("pub {}", parser::tts_to_string(&arg.raw_arg))
                                    .parse()
                                    .unwrap()
                            })
                            .collect();

                        let ix_data_trait = {
                            let name = method.raw_method.sig.ident.to_string();
                            let sighash_arr = sighash(SIGHASH_STATE_NAMESPACE, &name);
                            let sighash_tts: proc_macro2::TokenStream =
                                format!("{:?}", sighash_arr).parse().unwrap();
                            quote! {
                                impl anchor_lang::InstructionData for #ix_name_camel {
                                    fn data(&self) -> Vec<u8> {
                                        let mut d = #sighash_tts.to_vec();
                                        d.append(&mut self.try_to_vec().expect("Should always serialize"));
                                        d
                                    }
                                }
                            }
                        };

                        // If no args, output a "unit" variant instead of a struct variant.
                        if method.args.is_empty() {
                            quote! {
                                /// Anchor generated instruction.
                                #[derive(AnchorSerialize, AnchorDeserialize)]
                                pub struct #ix_name_camel;

                                #ix_data_trait
                            }
                        } else {
                            quote! {
                                /// Anchor generated instruction.
                                #[derive(AnchorSerialize, AnchorDeserialize)]
                                pub struct #ix_name_camel {
                                    #(#raw_args),*
                                }

                                #ix_data_trait
                            }
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };
    let variants: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let name = &ix.raw_method.sig.ident.to_string();
            let ix_name_camel =
                proc_macro2::Ident::new(&name.to_camel_case(), ix.raw_method.sig.ident.span());
            let raw_args: Vec<proc_macro2::TokenStream> = ix
                .args
                .iter()
                .map(|arg| {
                    format!("pub {}", parser::tts_to_string(&arg.raw_arg))
                        .parse()
                        .unwrap()
                })
                .collect();
            let ix_data_trait = {
                let sighash_arr = sighash(SIGHASH_GLOBAL_NAMESPACE, &name);
                let sighash_tts: proc_macro2::TokenStream =
                    format!("{:?}", sighash_arr).parse().unwrap();
                quote! {
                    impl anchor_lang::InstructionData for #ix_name_camel {
                        fn data(&self) -> Vec<u8> {
                            let mut d = #sighash_tts.to_vec();
                            d.append(&mut self.try_to_vec().expect("Should always serialize"));
                            d
                        }
                    }
                }
            };
            // If no args, output a "unit" variant instead of a struct variant.
            if ix.args.is_empty() {
                quote! {
                    /// Instruction.
                    #[derive(AnchorSerialize, AnchorDeserialize)]
                    pub struct #ix_name_camel;

                    #ix_data_trait
                }
            } else {
                quote! {
                    /// Instruction.
                    #[derive(AnchorSerialize, AnchorDeserialize)]
                    pub struct #ix_name_camel {
                        #(#raw_args),*
                    }

                    #ix_data_trait
                }
            }
        })
        .collect();

    quote! {
        /// An Anchor generated module containing the program's set of
        /// instructions, where each method handler in the `#[program]` mod is
        /// associated with a struct defining the input arguments to the
        /// method. These should be used directly, when one wants to serialize
        /// Anchor instruction data, for example, when speciying
        /// instructions on a client.
        pub mod instruction {
            use super::*;

            /// Instruction struct definitions for `#[state]` methods.
            pub mod state {
                use super::*;

                #ctor_variant
                #(#state_method_variants)*
            }

            #(#variants)*
        }
    }
}

fn generate_accounts(program: &Program) -> proc_macro2::TokenStream {
    let mut accounts = std::collections::HashSet::new();

    // Go through state accounts.
    if let Some(state) = &program.state {
        if let Some((_impl_block, methods)) = &state.impl_block_and_methods {
            for ix in methods {
                let anchor_ident = &ix.anchor_ident;
                // TODO: move to fn and share with accounts.rs.
                let macro_name = format!(
                    "__client_accounts_{}",
                    anchor_ident.to_string().to_snake_case()
                );
                accounts.insert(macro_name);
            }
        }
    }

    // Go through instruction accounts.
    for ix in &program.ixs {
        let anchor_ident = &ix.anchor_ident;
        // TODO: move to fn and share with accounts.rs.
        let macro_name = format!(
            "__client_accounts_{}",
            anchor_ident.to_string().to_snake_case()
        );
        accounts.insert(macro_name);
    }

    // Build the tokens from all accounts
    let account_structs: Vec<proc_macro2::TokenStream> = accounts
        .iter()
        .map(|macro_name: &String| {
            let macro_name: proc_macro2::TokenStream = macro_name.parse().unwrap();
            quote! {
                pub use crate::#macro_name::*;
            }
        })
        .collect();

    // TODO: calculate the account size and add it as a constant field to
    //       each struct here. This is convenient for Rust clients.

    quote! {
        /// An Anchor generated module, providing a set of structs
        /// mirroring the structs deriving `Accounts`, where each field is
        /// a `Pubkey`. This is useful for specifying accounts for a client.
        pub mod accounts {
            #(#account_structs)*
        }
    }
}

fn generate_cpi(program: &Program) -> proc_macro2::TokenStream {
    // Generate cpi methods for the state struct.
    // The Ctor is not exposed via CPI, since it is a one time use function.
    let state_cpi_methods: Vec<proc_macro2::TokenStream> = program
        .state
        .as_ref()
        .map(|state| {
            state
                .impl_block_and_methods
                .as_ref()
                .map(|(_, methods)| {
                    methods
                        .iter()
                        .map(|method: &StateIx| {
                            let accounts_ident = &method.anchor_ident;
                            let ix_variant = generate_ix_variant(
                                method.raw_method.sig.ident.to_string(),
                                &method.args,
                            );
                            let method_name = &method.ident;
                            let args: Vec<&syn::PatType> =
                                method.args.iter().map(|arg| &arg.raw_arg).collect();

                            quote! {
                                pub fn #method_name<'a, 'b, 'c, 'info>(
                                    ctx: CpiStateContext<'a, 'b, 'c, 'info, #accounts_ident<'info>>,
                                    #(#args),*
                                ) -> ProgramResult {
                                    let ix = {
                                        let ix = instruction::state::#ix_variant;
                                        let data = anchor_lang::InstructionData::data(&ix);
                                        let accounts = ctx.to_account_metas(None);
                                        anchor_lang::solana_program::instruction::Instruction {
                                            program_id: *ctx.program().key,
                                            accounts,
                                            data,
                                        }
                                    };
                                    let mut acc_infos = ctx.to_account_infos();
                                    anchor_lang::solana_program::program::invoke_signed(
                                        &ix,
                                        &acc_infos,
                                        ctx.signer_seeds(),
                                    )
                                }
                            }
                        })
                        .collect()
                })
                .unwrap_or(vec![])
        })
        .unwrap_or(vec![]);
    // Generate cpi methods for global methods.
    let global_cpi_methods: Vec<proc_macro2::TokenStream> = program
        .ixs
        .iter()
        .map(|ix| {
            let accounts_ident = &ix.anchor_ident;
            let cpi_method = {
                let ix_variant = generate_ix_variant(ix.raw_method.sig.ident.to_string(), &ix.args);
                let method_name = &ix.ident;
                let args: Vec<&syn::PatType> = ix.args.iter().map(|arg| &arg.raw_arg).collect();
                let name = &ix.raw_method.sig.ident.to_string();
                let sighash_arr = sighash(SIGHASH_GLOBAL_NAMESPACE, &name);
                let sighash_tts: proc_macro2::TokenStream =
                    format!("{:?}", sighash_arr).parse().unwrap();
                quote! {
                    pub fn #method_name<'a, 'b, 'c, 'info>(
                        ctx: CpiContext<'a, 'b, 'c, 'info, #accounts_ident<'info>>,
                        #(#args),*
                    ) -> ProgramResult {
                        let ix = {
                            let ix = instruction::#ix_variant;
                            let mut ix_data = AnchorSerialize::try_to_vec(&ix)
                                .map_err(|_| ProgramError::InvalidInstructionData)?;
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
            };

            cpi_method
        })
        .collect();
    quote! {
        #[cfg(feature = "cpi")]
        pub mod cpi {
            use super::*;

            pub mod state {
                use super::*;

                #(#state_cpi_methods)*
            }

            #(#global_cpi_methods)*
        }
    }
}

// We don't technically use sighash, because the input arguments aren't given.
// Rust doesn't have method overloading so no need to use the arguments.
// However, we do namespace methods in the preeimage so that we can use
// different traits with the same method name.
pub fn sighash(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{}::{}", namespace, name);

    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(&crate::hash::hash(preimage.as_bytes()).to_bytes()[..8]);
    sighash
}

fn sighash_ctor() -> [u8; 8] {
    let namespace = SIGHASH_STATE_NAMESPACE;
    let preimage = format!("{}::new", namespace);

    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(&crate::hash::hash(preimage.as_bytes()).to_bytes()[..8]);
    sighash
}
