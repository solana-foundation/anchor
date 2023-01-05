use crate::codegen::program::common::*;
use crate::Program;
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
                    anchor_lang::idl::IdlInstruction::Resize { data_len } => {
                        let mut bumps = std::collections::BTreeMap::new();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            anchor_lang::idl::IdlResizeAccount::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_resize_account(program_id, &mut accounts, data_len)?;
                        accounts.exit(program_id)?;
                    },
                    anchor_lang::idl::IdlInstruction::Close => {
                        let mut bumps = std::collections::BTreeMap::new();
                        let mut reallocs = std::collections::BTreeSet::new();
                        let mut accounts =
                            anchor_lang::idl::IdlCloseAccount::try_accounts(program_id, &mut accounts, &[], &mut bumps, &mut reallocs)?;
                        __idl_close_account(program_id, &mut accounts)?;
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
                let space = std::cmp::min(8 + 32 + 4 + data_len as usize, 10_000);
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
            pub fn __idl_resize_account(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlResizeAccount,
                data_len: u64,
            ) -> anchor_lang::Result<()> {
                #[cfg(not(feature = "no-log-ix-name"))]
                anchor_lang::prelude::msg!("Instruction: IdlResizeAccount");

                let data_len: usize = data_len as usize;

                // We're not going to support increasing the size of accounts that already contain data
                // because that would be messy and possibly dangerous
                if accounts.idl.data_len != 0 {
                    return Err(anchor_lang::error::ErrorCode::IdlAccountNotEmpty.into());
                }

                let new_account_space = accounts.idl.to_account_info().data_len().checked_add(std::cmp::min(
                    data_len
                        .checked_sub(accounts.idl.to_account_info().data_len())
                        .expect("data_len should always be >= the current account space"),
                    10_000,
                ))
                .unwrap();

                if new_account_space > accounts.idl.to_account_info().data_len() {
                    let sysvar_rent = Rent::get()?;
                    let new_rent_minimum = sysvar_rent.minimum_balance(new_account_space);
                    anchor_lang::system_program::transfer(
                        anchor_lang::context::CpiContext::new(
                            accounts.system_program.to_account_info(),
                            anchor_lang::system_program::Transfer {
                                from: accounts.authority.to_account_info(),
                                to: accounts.idl.to_account_info().clone(),
                            },
                        ),
                        new_rent_minimum
                            .checked_sub(accounts.idl.to_account_info().lamports())
                            .unwrap(),
                    )?;
                    accounts.idl.to_account_info().realloc(new_account_space, false)?;
                }

                Ok(())

            }

            #[inline(never)]
            pub fn __idl_close_account(
                program_id: &Pubkey,
                accounts: &mut anchor_lang::idl::IdlCloseAccount,
            ) -> anchor_lang::Result<()> {
                #[cfg(not(feature = "no-log-ix-name"))]
                anchor_lang::prelude::msg!("Instruction: IdlCloseAccount");

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

                let prev_len: usize = ::std::convert::TryInto::<usize>::try_into(accounts.idl.data_len).unwrap();
                let new_len: usize = prev_len + idl_data.len();
                accounts.idl.data_len = accounts.idl.data_len.checked_add(::std::convert::TryInto::<u32>::try_into(idl_data.len()).unwrap()).unwrap();

                use anchor_lang::idl::IdlTrailingData;
                let mut idl_bytes = accounts.idl.trailing_data_mut();
                let idl_expansion = &mut idl_bytes[prev_len..new_len];
                require_eq!(idl_expansion.len(), idl_data.len());
                idl_expansion.copy_from_slice(&idl_data[..]);

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

                accounts.idl.data_len = accounts.buffer.data_len;

                use anchor_lang::idl::IdlTrailingData;
                let buffer_len = ::std::convert::TryInto::<usize>::try_into(accounts.buffer.data_len).unwrap();
                let mut target = accounts.idl.trailing_data_mut();
                let source = &accounts.buffer.trailing_data()[..buffer_len];
                require_gte!(target.len(), buffer_len);
                target[..buffer_len].copy_from_slice(source);
                // zero the remainder of target?

                Ok(())
            }
        }
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
