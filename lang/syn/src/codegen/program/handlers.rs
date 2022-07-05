use crate::codegen::program::common::*;
use crate::{Program, State};
use heck::CamelCase;
use quote::{quote, ToTokens};

// Generate non-inlined wrappers for each instruction handler, since Solana's
// BPF max stack size can't handle reasonable sized dispatch trees without doing
// so.
pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let program_name = &program.name;
    let non_inlined_idl: proc_macro2::TokenStream = {
        quote! {
            // Entry for all IDL related instructions. Use the "no-idl" feature
            // to eliminate this code, for example, if one wants to make the
            // IDL no longer mutable or if one doesn't want to store the IDL
            // on chain.
            #[inline(never)]
            #[cfg(not(feature = "no-idl"))]
            pub fn __idl_dispatch(program_id: &Pubkey, accounts: &[AccountInfo], idl_ix_data: &[u8]) -> anchor_lang::Result<()> {
                let mut accounts = accounts;
                let mut data: &[u8] = idl_ix_data;

                let ix = anchor_lang::idl::IdlInstruction::deserialize(&mut data)
                    .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;

                match ix {
                    anchor_lang::idl::IdlInstruction::Create { data_len } => {
                        let mut bumps = std::collections::BTreeMap::new();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            anchor_lang::idl::IdlCreateAccounts::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_create_account(program_id, &mut accounts, data_len)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::CreateBuffer => {
                        let mut bumps = std::collections::BTreeMap::new();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            anchor_lang::idl::IdlCreateBuffer::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_create_buffer(program_id, &mut accounts)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::Write { data } => {
                        let mut bumps = std::collections::BTreeMap::new();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            anchor_lang::idl::IdlAccounts::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_write(program_id, &mut accounts, data)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::SetAuthority { new_authority } => {
                        let mut bumps = std::collections::BTreeMap::new();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            anchor_lang::idl::IdlAccounts::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_set_authority(program_id, &mut accounts, new_authority)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::SetBuffer => {
                        let mut bumps = std::collections::BTreeMap::new();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            anchor_lang::idl::IdlSetBuffer::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_set_buffer(program_id, &mut accounts)?;
                        accounts.exit(program_id)?;
                    },
                }
                Ok(())
            }

            #[inline(never)]
            #[cfg(feature = "no-idl")]
            pub fn __idl_dispatch(program_id: &Pubkey, accounts: &[AccountInfo], idl_ix_data: &[u8]) -> anchor_lang::Result<()> {
                Err(anchor_lang::error::ErrorCode::IdlInstructionStub.into())
            }

            // One time IDL account initializer. Will faill on subsequent
            // invocations.
            #[inline(never)]
            pub fn __idl_create_account(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlCreateAccounts,
                data_len: u64,
            ) -> anchor_lang::Result<()> {
                #[cfg(not(feature = "no-log-ix-name"))]
                anchor_lang::prelude::msg!("Instruction: IdlCreateAccount");

                if program_id != accounts.program.key {
                    return Err(anchor_lang::error::ErrorCode::IdlInstructionInvalidProgram.into());
                }
                // Create the IDL's account.
                let from = accounts.from.key;
                let (base, nonce) = Pubkey::find_program_address(&[], program_id);
                let seed = anchor_lang::idl::IdlAccount::seed();
                let owner = accounts.program.key;
                let to = Pubkey::create_with_seed(&base, seed, owner).unwrap();
                // Space: account discriminator || authority pubkey || vec len || vec data
                let space = 8 + 32 + 4 + data_len as usize;
                let rent = Rent::get()?;
                let lamports = rent.minimum_balance(space);
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
            ) -> anchor_lang::Result<()> {
                #[cfg(not(feature = "no-log-ix-name"))]
                anchor_lang::prelude::msg!("Instruction: IdlCreateBuffer");

                let mut buffer = &mut accounts.buffer;
                buffer.authority = *accounts.authority.key;
                Ok(())
            }

            #[inline(never)]
            pub fn __idl_write(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlAccounts,
                idl_data: Vec<u8>,
            ) -> anchor_lang::Result<()> {
                #[cfg(not(feature = "no-log-ix-name"))]
                anchor_lang::prelude::msg!("Instruction: IdlWrite");

                let mut idl = &mut accounts.idl;
                idl.data.extend(idl_data);
                Ok(())
            }

            #[inline(never)]
            pub fn __idl_set_authority(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlAccounts,
                new_authority: Pubkey,
            ) -> anchor_lang::Result<()> {
                #[cfg(not(feature = "no-log-ix-name"))]
                anchor_lang::prelude::msg!("Instruction: IdlSetAuthority");

                accounts.idl.authority = new_authority;
                Ok(())
            }

            #[inline(never)]
            pub fn __idl_set_buffer(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlSetBuffer,
            ) -> anchor_lang::Result<()> {
                #[cfg(not(feature = "no-log-ix-name"))]
                anchor_lang::prelude::msg!("Instruction: IdlSetBuffer");

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
                let ctor_untyped_args = generate_ctor_args(state);
                let name = &state.strct.ident;
                let mod_name = &program.name;
                let variant_arm = generate_ctor_variant(state);
                let ix_name: proc_macro2::TokenStream =
                    generate_ctor_variant_name().parse().unwrap();
                let ix_name_log = format!("Instruction: {}", ix_name);
                if state.is_zero_copy {
                    quote! {
                        // One time state account initializer. Will faill on subsequent
                        // invocations.
                        #[inline(never)]
                        pub fn __ctor(program_id: &Pubkey, accounts: &[AccountInfo], ix_data: &[u8]) -> anchor_lang::Result<()> {
                            #[cfg(not(feature = "no-log-ix-name"))]
                            anchor_lang::prelude::msg!(#ix_name_log);

                            // Deserialize instruction data.
                            let ix = instruction::state::#ix_name::deserialize(&mut &ix_data[..])
                                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                            let instruction::state::#variant_arm = ix;

                            let mut __bumps = std::collections::BTreeMap::new();
                            let mut __reallocs = std::collections::BTreeSet::new();

                            // Deserialize accounts.
                            let mut remaining_accounts: &[AccountInfo] = accounts;
                            let ctor_accounts =
                            anchor_lang::__private::Ctor::try_accounts(program_id, &mut remaining_accounts, &[], &mut __bumps, &mut __reallocs)?;
                            let mut ctor_user_def_accounts =
                            #anchor_ident::try_accounts(program_id, &mut remaining_accounts, ix_data, &mut __bumps, &mut __reallocs)?;

                            // Create the solana account for the ctor data.
                            let from = ctor_accounts.from.key;
                            let (base, nonce) = Pubkey::find_program_address(&[], ctor_accounts.program.key);
                            let seed = anchor_lang::__private::PROGRAM_STATE_SEED;
                            let owner = ctor_accounts.program.key;
                            let to = Pubkey::create_with_seed(&base, seed, owner).unwrap();
                            let space = 8 + std::mem::size_of::<#name>();
                            let rent = Rent::get()?;
                            let lamports = rent.minimum_balance(std::convert::TryInto::try_into(space).unwrap());
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
                            let loader: anchor_lang::accounts::loader::Loader<#mod_name::#name> = anchor_lang::accounts::loader::Loader::try_from_unchecked(program_id, &ctor_accounts.to)?;

                            // Invoke the ctor in a new lexical scope so that
                            // the zero-copy RefMut gets dropped. Required
                            // so that we can subsequently run the exit routine.
                            {
                                let mut instance = loader.load_init()?;
                                instance.new(
                                    anchor_lang::context::Context::new(
                                        program_id,
                                        &mut ctor_user_def_accounts,
                                        remaining_accounts,
                                        __bumps,
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
                        pub fn __ctor(program_id: &Pubkey, accounts: &[AccountInfo], ix_data: &[u8]) -> anchor_lang::Result<()> {
                            #[cfg(not(feature = "no-log-ix-name"))]
                            anchor_lang::prelude::msg!(#ix_name_log);

                            // Deserialize instruction data.
                            let ix = instruction::state::#ix_name::deserialize(&mut &ix_data[..])
                                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                            let instruction::state::#variant_arm = ix;

                            let mut __bumps = std::collections::BTreeMap::new();
                            let mut __reallocs = std::collections::BTreeSet::new();

                            // Deserialize accounts.
                            let mut remaining_accounts: &[AccountInfo] = accounts;
                            let ctor_accounts =
                            anchor_lang::__private::Ctor::try_accounts(program_id, &mut remaining_accounts, &[], &mut __bumps, &mut __reallocs)?;
                            let mut ctor_user_def_accounts =
                            #anchor_ident::try_accounts(program_id, &mut remaining_accounts, ix_data, &mut __bumps, &mut __reallocs)?;

                            // Invoke the ctor.
                            let instance = #mod_name::#name::new(
                                anchor_lang::context::Context::new(
                                    program_id,
                                    &mut ctor_user_def_accounts,
                                    remaining_accounts,
                                    __bumps,
                                ),
                                #(#ctor_untyped_args),*
                            )?;

                            // Create the solana account for the ctor data.
                            let from = ctor_accounts.from.key;
                            let (base, nonce) = Pubkey::find_program_address(&[], ctor_accounts.program.key);
                            let seed = anchor_lang::accounts::state::ProgramState::<#name>::seed();
                            let owner = ctor_accounts.program.key;
                            let to = Pubkey::create_with_seed(&base, seed, owner).unwrap();
                            let space = anchor_lang::__private::AccountSize::size(&instance)?;
                            let rent = Rent::get()?;
                            let lamports = rent.minimum_balance(std::convert::TryInto::try_into(space).unwrap());
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
                        let ix_arg_names: Vec<&syn::Ident> =
                            ix.args.iter().map(|arg| &arg.name).collect();
                        let private_ix_method_name: proc_macro2::TokenStream = {
                            let n = format!("__{}", &ix.raw_method.sig.ident.to_string());
                            n.parse().unwrap()
                        };
                        let ix_method_name = &ix.raw_method.sig.ident;
                        let state_ty: proc_macro2::TokenStream = state.name.parse().unwrap();
                        let anchor_ident = &ix.anchor_ident;
                        let name = &state.strct.ident;
                        let mod_name = &program.name;

                        let variant_arm =
                            generate_ix_variant(ix.raw_method.sig.ident.to_string(), &ix.args);
                        let ix_name = generate_ix_variant_name(ix.raw_method.sig.ident.to_string());
                        let ix_name_log = format!("Instruction: {}", ix_name);

                        if state.is_zero_copy {
                            quote! {
                                #[inline(never)]
                                pub fn #private_ix_method_name(
                                    program_id: &Pubkey,
                                    accounts: &[AccountInfo],
                                    ix_data: &[u8],
                                ) -> anchor_lang::Result<()> {
                                    #[cfg(not(feature = "no-log-ix-name"))]
                                    anchor_lang::prelude::msg!(#ix_name_log);

                                    // Deserialize instruction.
                                    let ix = instruction::state::#ix_name::deserialize(&mut &ix_data[..])
                                        .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                                    let instruction::state::#variant_arm = ix;

                                    // Bump collector.
                                    let mut __bumps = std::collections::BTreeMap::new();

                                    // Realloc tracker
                                    let mut __reallocs= std::collections::BTreeSet::new();

                                    // Load state.
                                    let mut remaining_accounts: &[AccountInfo] = accounts;
                                    if remaining_accounts.is_empty() {
                                        return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
                                    }
                                    let loader: anchor_lang::accounts::loader::Loader<#mod_name::#name> = anchor_lang::accounts::loader::Loader::try_accounts(program_id, &mut remaining_accounts, &[], &mut __bumps, &mut __reallocs)?;

                                    // Deserialize accounts.
                                    let mut accounts = #anchor_ident::try_accounts(
                                        program_id,
                                        &mut remaining_accounts,
                                        ix_data,
                                        &mut __bumps,
                                        &mut __reallocs,
                                    )?;
                                    let ctx =
                                        anchor_lang::context::Context::new(
                                            program_id,
                                            &mut accounts,
                                            remaining_accounts,
                                            __bumps,
                                        );

                                    // Execute user defined function.
                                    {
                                        let mut state = loader.load_mut()?;
                                        state.#ix_method_name(
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
                                pub fn #private_ix_method_name(
                                    program_id: &Pubkey,
                                    accounts: &[AccountInfo],
                                    ix_data: &[u8],
                                ) -> anchor_lang::Result<()> {
                                    #[cfg(not(feature = "no-log-ix-name"))]
                                    anchor_lang::prelude::msg!(#ix_name_log);

                                    // Deserialize instruction.
                                    let ix = instruction::state::#ix_name::deserialize(&mut &ix_data[..])
                                        .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                                    let instruction::state::#variant_arm = ix;

                                    // Bump collector.
                                    let mut __bumps = std::collections::BTreeMap::new();

                                    // Realloc tracker.
                                    let mut __reallocs = std::collections::BTreeSet::new();

                                    // Load state.
                                    let mut remaining_accounts: &[AccountInfo] = accounts;
                                    if remaining_accounts.is_empty() {
                                        return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
                                    }
                                    let mut state: anchor_lang::accounts::state::ProgramState<#state_ty> = anchor_lang::accounts::state::ProgramState::try_accounts(
                                        program_id,
                                        &mut remaining_accounts,
                                        &[],
                                        &mut __bumps,
                                        &mut __reallocs,
                                    )?;

                                    // Deserialize accounts.
                                    let mut accounts = #anchor_ident::try_accounts(
                                        program_id,
                                        &mut remaining_accounts,
                                        ix_data,
                                        &mut __bumps,
                                        &mut __reallocs,
                                    )?;
                                    let ctx =
                                        anchor_lang::context::Context::new(
                                            program_id,
                                            &mut accounts,
                                            remaining_accounts,
                                            __bumps
                                        );

                                    // Execute user defined function.
                                    state.#ix_method_name(
                                        ctx,
                                        #(#ix_arg_names),*
                                    )?;

                                    // Serialize the state and save it to storage.
                                    accounts.exit(program_id)?;
                                    let acc_info = state.to_account_info();
                                    let mut data = acc_info.try_borrow_mut_data()?;
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
                                // Easy to implement. Just need to write a test.
                                // Feel free to open a PR.
                                assert!(!state.is_zero_copy, "Trait implementations not yet implemented for zero copy state structs. Please file an issue.");

                                let ix_arg_names: Vec<&syn::Ident> =
                                    ix.args.iter().map(|arg| &arg.name).collect();
                                let private_ix_method_name: proc_macro2::TokenStream = {
                                    let n = format!("__{}_{}", iface.trait_name, &ix.raw_method.sig.ident.to_string());
                                    n.parse().unwrap()
                                };
                                let ix_method_name = &ix.raw_method.sig.ident;
                                let state_ty: proc_macro2::TokenStream = state.name.parse().unwrap();
                                let anchor_ident = &ix.anchor_ident;
                                let ix_name = generate_ix_variant_name(ix.raw_method.sig.ident.to_string());
                                let ix_name_log = format!("Instruction: {}", ix_name);

                                let raw_args: Vec<&syn::PatType> = ix
                                    .args
                                    .iter()
                                    .map(|arg: &crate::IxArg| &arg.raw_arg)
                                    .collect();
                                let args_struct = {
                                    if ix.args.is_empty() {
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

                                let deserialize_instruction = quote! {
                                    #args_struct
                                    let ix = Args::deserialize(&mut &ix_data[..])
                                        .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                                    let Args {
                                        #(#ix_arg_names),*
                                    } = ix;
                                };

                                if ix.has_receiver {
                                    quote! {
                                        #[inline(never)]
                                        pub fn #private_ix_method_name(
                                            program_id: &Pubkey,
                                            accounts: &[AccountInfo],
                                            ix_data: &[u8],
                                        ) -> anchor_lang::Result<()> {
                                            #[cfg(not(feature = "no-log-ix-name"))]
                                            anchor_lang::prelude::msg!(#ix_name_log);

                                            // Deserialize instruction.
                                            #deserialize_instruction

                                            // Bump collector.
                                            let mut __bumps = std::collections::BTreeMap::new();

                                            // Realloc tracker.
                                            let mut __reallocs= std::collections::BTreeSet::new();

                                            // Deserialize the program state account.
                                            let mut remaining_accounts: &[AccountInfo] = accounts;
                                            if remaining_accounts.is_empty() {
                                                return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
                                            }
                                            let mut state: anchor_lang::accounts::state::ProgramState<#state_ty> = anchor_lang::accounts::state::ProgramState::try_accounts(
                                                program_id,
                                                &mut remaining_accounts,
                                                &[],
                                                &mut __bumps,
                                                &mut __reallocs,
                                            )?;

                                            // Deserialize accounts.
                                            let mut accounts = #anchor_ident::try_accounts(
                                                program_id,
                                                &mut remaining_accounts,
                                                ix_data,
                                                &mut __bumps,
                                                &mut __reallocs,
                                            )?;
                                            let ctx =
                                                anchor_lang::context::Context::new(
                                                    program_id,
                                                    &mut accounts,
                                                    remaining_accounts,
                                                    __bumps,
                                                );

                                            // Execute user defined function.
                                            state.#ix_method_name(
                                                ctx,
                                                #(#ix_arg_names),*
                                            )?;

                                            // Exit procedures.
                                            accounts.exit(program_id)?;
                                            let acc_info = state.to_account_info();
                                            let mut data = acc_info.try_borrow_mut_data()?;
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
                                        pub fn #private_ix_method_name(
                                            program_id: &Pubkey,
                                            accounts: &[AccountInfo],
                                            ix_data: &[u8],
                                        ) -> anchor_lang::Result<()> {
                                            #[cfg(not(feature = "no-log-ix-name"))]
                                            anchor_lang::prelude::msg!(#ix_name_log);

                                            // Deserialize instruction.
                                            #deserialize_instruction

                                            // Bump collector.
                                            let mut __bumps = std::collections::BTreeMap::new();

                                            let mut __reallocs = std::collections::BTreeSet::new();

                                            // Deserialize accounts.
                                            let mut remaining_accounts: &[AccountInfo] = accounts;
                                            let mut accounts = #anchor_ident::try_accounts(
                                                program_id,
                                                &mut remaining_accounts,
                                                ix_data,
                                                &mut __bumps,
                                                &mut __reallocs,
                                            )?;

                                            // Execute user defined function.
                                            #state_name::#ix_method_name(
                                                anchor_lang::context::Context::new(
                                                    program_id,
                                                    &mut accounts,
                                                    remaining_accounts,
                                                    __bumps
                                                ),
                                                #(#ix_arg_names),*
                                            )?;

                                            // Exit procedure.
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
            let ix_arg_names: Vec<&syn::Ident> = ix.args.iter().map(|arg| &arg.name).collect();
            let ix_name = generate_ix_variant_name(ix.raw_method.sig.ident.to_string());
            let ix_method_name = &ix.raw_method.sig.ident;
            let anchor = &ix.anchor_ident;
            let variant_arm = generate_ix_variant(ix.raw_method.sig.ident.to_string(), &ix.args);
            let ix_name_log = format!("Instruction: {}", ix_name);
            let ret_type = &ix.returns.ty.to_token_stream();
            let maybe_set_return_data = match ret_type.to_string().as_str() {
                "()" => quote! {},
                _ => quote! {
                    anchor_lang::solana_program::program::set_return_data(&result.try_to_vec().unwrap());
                },
            };
            quote! {
                #[inline(never)]
                pub fn #ix_method_name(
                    program_id: &Pubkey,
                    accounts: &[AccountInfo],
                    ix_data: &[u8],
                ) -> anchor_lang::Result<()> {
                    #[cfg(not(feature = "no-log-ix-name"))]
                    anchor_lang::prelude::msg!(#ix_name_log);

                    // Deserialize data.
                    let ix = instruction::#ix_name::deserialize(&mut &ix_data[..])
                        .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
                    let instruction::#variant_arm = ix;

                    // Bump collector.
                    let mut __bumps = std::collections::BTreeMap::new();

                    let mut __reallocs = std::collections::BTreeSet::new();

                    // Deserialize accounts.
                    let mut remaining_accounts: &[AccountInfo] = accounts;
                    let mut accounts = #anchor::try_accounts(
                        program_id,
                        &mut remaining_accounts,
                        ix_data,
                        &mut __bumps,
                        &mut __reallocs,
                    )?;

                    // Invoke user defined handler.
                    let result = #program_name::#ix_method_name(
                        anchor_lang::context::Context::new(
                            program_id,
                            &mut accounts,
                            remaining_accounts,
                            __bumps,
                        ),
                        #(#ix_arg_names),*
                    )?;

                    // Maybe set Solana return data.
                    #maybe_set_return_data

                    // Exit routine.
                    accounts.exit(program_id)
                }
            }
        })
        .collect();

    quote! {
        /// Create a private module to not clutter the program's namespace.
        /// Defines an entrypoint for each individual instruction handler
        /// wrapper.
        mod __private {
            use super::*;
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
}

fn generate_ix_variant_name(name: String) -> proc_macro2::TokenStream {
    let n = name.to_camel_case();
    n.parse().unwrap()
}

fn generate_ctor_variant_name() -> String {
    "New".to_string()
}

fn generate_ctor_variant(state: &State) -> proc_macro2::TokenStream {
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
