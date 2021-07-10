use crate::{
    CompositeField, Constraint, ConstraintAddress, ConstraintAssociatedGroup, ConstraintClose,
    ConstraintExecutable, ConstraintGroup, ConstraintHasOne, ConstraintInit, ConstraintLiteral,
    ConstraintMut, ConstraintOwner, ConstraintRaw, ConstraintRentExempt, ConstraintSeedsGroup,
    ConstraintSigner, ConstraintState, Field, PdaKind, Ty,
};
use proc_macro2_diagnostics::SpanDiagnosticExt;
use quote::quote;
use syn::Expr;

pub fn generate(f: &Field) -> proc_macro2::TokenStream {
    let checks: Vec<proc_macro2::TokenStream> = linearize(&f.constraints)
        .iter()
        .map(|c| generate_constraint(f, c))
        .collect();
    quote! {
        #(#checks)*
    }
}

pub fn generate_composite(f: &CompositeField) -> proc_macro2::TokenStream {
    let checks: Vec<proc_macro2::TokenStream> = linearize(&f.constraints)
        .iter()
        .filter_map(|c| match c {
            Constraint::Raw(_) => Some(c),
            Constraint::Literal(_) => Some(c),
            _ => panic!("Invariant violation: composite constraints can only be raw or literals"),
        })
        .map(|c| generate_constraint_composite(f, c))
        .collect();
    quote! {
        #(#checks)*
    }
}

// Linearizes the constraint group so that constraints with dependencies
// run after those without.
//
// The associated cosntraint should always be first since it may also create
// an account with a PDA.
pub fn linearize(c_group: &ConstraintGroup) -> Vec<Constraint> {
    let ConstraintGroup {
        init,
        mutable,
        signer,
        has_one,
        literal,
        raw,
        owner,
        rent_exempt,
        seeds,
        executable,
        state,
        associated,
        close,
        address,
    } = c_group.clone();

    let mut constraints = Vec::new();

    if let Some(c) = associated {
        constraints.push(Constraint::AssociatedGroup(c));
    }
    if let Some(c) = seeds {
        constraints.push(Constraint::Seeds(c));
    }
    if let Some(c) = init {
        constraints.push(Constraint::Init(c));
    }
    if let Some(c) = mutable {
        constraints.push(Constraint::Mut(c));
    }
    if let Some(c) = signer {
        constraints.push(Constraint::Signer(c));
    }
    constraints.append(&mut has_one.into_iter().map(Constraint::HasOne).collect());
    constraints.append(&mut literal.into_iter().map(Constraint::Literal).collect());
    constraints.append(&mut raw.into_iter().map(Constraint::Raw).collect());
    if let Some(c) = owner {
        constraints.push(Constraint::Owner(c));
    }
    if let Some(c) = rent_exempt {
        constraints.push(Constraint::RentExempt(c));
    }
    if let Some(c) = executable {
        constraints.push(Constraint::Executable(c));
    }
    if let Some(c) = state {
        constraints.push(Constraint::State(c));
    }
    if let Some(c) = close {
        constraints.push(Constraint::Close(c));
    }
    if let Some(c) = address {
        constraints.push(Constraint::Address(c));
    }
    constraints
}

fn generate_constraint(f: &Field, c: &Constraint) -> proc_macro2::TokenStream {
    match c {
        Constraint::Init(c) => generate_constraint_init(f, c),
        Constraint::Mut(c) => generate_constraint_mut(f, c),
        Constraint::HasOne(c) => generate_constraint_has_one(f, c),
        Constraint::Signer(c) => generate_constraint_signer(f, c),
        Constraint::Literal(c) => generate_constraint_literal(c),
        Constraint::Raw(c) => generate_constraint_raw(c),
        Constraint::Owner(c) => generate_constraint_owner(f, c),
        Constraint::RentExempt(c) => generate_constraint_rent_exempt(f, c),
        Constraint::Seeds(c) => generate_constraint_seeds(f, c),
        Constraint::Executable(c) => generate_constraint_executable(f, c),
        Constraint::State(c) => generate_constraint_state(f, c),
        Constraint::AssociatedGroup(c) => generate_constraint_associated(f, c),
        Constraint::Close(c) => generate_constraint_close(f, c),
        Constraint::Address(c) => generate_constraint_address(f, c),
    }
}

fn generate_constraint_composite(_f: &CompositeField, c: &Constraint) -> proc_macro2::TokenStream {
    match c {
        Constraint::Raw(c) => generate_constraint_raw(c),
        Constraint::Literal(c) => generate_constraint_literal(c),
        _ => panic!("Invariant violation"),
    }
}

fn generate_constraint_address(f: &Field, c: &ConstraintAddress) -> proc_macro2::TokenStream {
    let field = &f.ident;
    let addr = &c.address;
    quote! {
        if #field.to_account_info().key != &#addr {
            return Err(anchor_lang::__private::ErrorCode::ConstraintAddress.into());
        }
    }
}

pub fn generate_constraint_init(_f: &Field, _c: &ConstraintInit) -> proc_macro2::TokenStream {
    quote! {}
}

pub fn generate_constraint_close(f: &Field, c: &ConstraintClose) -> proc_macro2::TokenStream {
    let field = &f.ident;
    let target = &c.sol_dest;
    quote! {
        if #field.to_account_info().key == #target.to_account_info().key {
            return Err(anchor_lang::__private::ErrorCode::ConstraintClose.into());
        }
    }
}

pub fn generate_constraint_mut(f: &Field, _c: &ConstraintMut) -> proc_macro2::TokenStream {
    let ident = &f.ident;
    quote! {
        if !#ident.to_account_info().is_writable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintMut.into());
        }
    }
}

pub fn generate_constraint_has_one(f: &Field, c: &ConstraintHasOne) -> proc_macro2::TokenStream {
    let target = c.join_target.clone();
    let ident = &f.ident;
    let field = match &f.ty {
        Ty::Loader(_) => quote! {#ident.load()?},
        _ => quote! {#ident},
    };
    quote! {
        if &#field.#target != #target.to_account_info().key {
            return Err(anchor_lang::__private::ErrorCode::ConstraintHasOne.into());
        }
    }
}

pub fn generate_constraint_signer(f: &Field, _c: &ConstraintSigner) -> proc_macro2::TokenStream {
    let ident = &f.ident;
    let info = match f.ty {
        Ty::AccountInfo => quote! { #ident },
        Ty::ProgramAccount(_) => quote! { #ident.to_account_info() },
        _ => panic!("Invalid syntax: signer cannot be specified."),
    };
    quote! {
        // Don't enforce on CPI, since usually a program is signing and so
        // the `try_accounts` deserializatoin will fail *if* the one
        // tries to manually invoke it.
        //
        // This check will be performed on the other end of the invocation.
        if cfg!(not(feature = "cpi")) {
            if !#info.to_account_info().is_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSigner.into());
            }
        }
    }
}

pub fn generate_constraint_literal(c: &ConstraintLiteral) -> proc_macro2::TokenStream {
    let lit: proc_macro2::TokenStream = {
        let lit = &c.lit;
        let constraint = lit.value().replace("\"", "");
        let message = format!(
            "Deprecated. Should be used with constraint: #[account(constraint = {})]",
            constraint,
        );
        lit.span().warning(message).emit_as_item_tokens();
        constraint.parse().unwrap()
    };
    quote! {
        if !(#lit) {
            return Err(anchor_lang::__private::ErrorCode::Deprecated.into());
        }
    }
}

pub fn generate_constraint_raw(c: &ConstraintRaw) -> proc_macro2::TokenStream {
    let raw = &c.raw;
    quote! {
        if !(#raw) {
            return Err(anchor_lang::__private::ErrorCode::ConstraintRaw.into());
        }
    }
}

pub fn generate_constraint_owner(f: &Field, c: &ConstraintOwner) -> proc_macro2::TokenStream {
    let ident = &f.ident;
    let owner_target = c.owner_target.clone();
    quote! {
        if #ident.to_account_info().owner != #owner_target.to_account_info().key {
            return Err(anchor_lang::__private::ErrorCode::ConstraintOwner.into());
        }
    }
}

pub fn generate_constraint_rent_exempt(
    f: &Field,
    c: &ConstraintRentExempt,
) -> proc_macro2::TokenStream {
    let ident = &f.ident;
    let info = quote! {
        #ident.to_account_info()
    };
    match c {
        ConstraintRentExempt::Skip => quote! {},
        ConstraintRentExempt::Enforce => quote! {
            if !rent.is_exempt(#info.lamports(), #info.try_data_len()?) {
                return Err(anchor_lang::__private::ErrorCode::ConstraintRentExempt.into());
            }
        },
    }
}

pub fn generate_constraint_seeds(f: &Field, c: &ConstraintSeedsGroup) -> proc_macro2::TokenStream {
    if c.is_init {
        generate_constraint_seeds_init(f, c)
    } else {
        generate_constraint_seeds_address(f, c)
    }
}

fn generate_constraint_seeds_init(f: &Field, c: &ConstraintSeedsGroup) -> proc_macro2::TokenStream {
    let payer = {
        let p = &c.payer;
        quote! {
            let payer = #p.to_account_info();
        }
    };
    let seeds_constraint = generate_constraint_seeds_address(f, c);
    let seeds_with_nonce = {
        let s = &c.seeds;
        match c.bump.as_ref() {
            None => quote! {
                [#s]
            },
            Some(b) => quote! {
                [#s, &[#b]]
            },
        }
    };
    generate_pda(
        f,
        seeds_constraint,
        seeds_with_nonce,
        payer,
        &c.space,
        false,
        &c.kind,
    )
}

fn generate_constraint_seeds_address(
    f: &Field,
    c: &ConstraintSeedsGroup,
) -> proc_macro2::TokenStream {
    let name = &f.ident;

    // If the bump is provided on *initialization*, then force it to be the
    // canonical nonce.
    if c.is_init && c.bump.is_some() {
        let s = &c.seeds;
        let b = c.bump.as_ref().unwrap();
        quote! {
            let (__program_signer, __bump) = anchor_lang::solana_program::pubkey::Pubkey::find_program_address(
                &[#s],
                program_id,
            );
            if #name.to_account_info().key != &__program_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
            }
            if __bump != #b {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
            }
        }
    } else {
        let seeds = match c.bump.as_ref() {
            None => {
                let s = &c.seeds;
                quote! {
                    [#s]
                }
            }
            Some(b) => {
                let s = &c.seeds;
                quote! {
                    [#s, &[#b]]
                }
            }
        };
        quote! {
            let __program_signer = Pubkey::create_program_address(
                &#seeds,
                program_id,
            ).map_err(|_| anchor_lang::__private::ErrorCode::ConstraintSeeds)?;
            if #name.to_account_info().key != &__program_signer {
                return Err(anchor_lang::__private::ErrorCode::ConstraintSeeds.into());
            }
        }
    }
}

pub fn generate_constraint_associated(
    f: &Field,
    c: &ConstraintAssociatedGroup,
) -> proc_macro2::TokenStream {
    if c.is_init {
        generate_constraint_associated_init(f, c)
    } else {
        generate_constraint_associated_seeds(f, c)
    }
}

pub fn generate_constraint_associated_init(
    f: &Field,
    c: &ConstraintAssociatedGroup,
) -> proc_macro2::TokenStream {
    let associated_target = c.associated_target.clone();
    let payer = match &c.payer {
        None => quote! {
            let payer = #associated_target.to_account_info();
        },
        Some(p) => quote! {
            let payer = #p.to_account_info();
        },
    };
    let seeds_constraint = generate_constraint_associated_seeds(f, c);
    let seeds_with_nonce = {
        if c.associated_seeds.is_empty() {
            quote! {
                [
                    &b"anchor"[..],
                    #associated_target.to_account_info().key.as_ref(),
                    &[nonce],
                ]
            }
        } else {
            let seeds = to_seeds_tts(&c.associated_seeds);
            quote! {
                [
                    &b"anchor"[..],
                    #associated_target.to_account_info().key.as_ref(),
                    #seeds
                    &[nonce],
                ]
            }
        }
    };

    generate_pda(
        f,
        seeds_constraint,
        seeds_with_nonce,
        payer,
        &c.space,
        true,
        &c.kind,
    )
}

fn parse_ty(f: &Field) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, bool) {
    match &f.ty {
        Ty::ProgramAccount(ty) => {
            let ident = &ty.account_type_path;
            (
                quote! {
                    #ident
                },
                quote! {
                    anchor_lang::ProgramAccount
                },
                false,
            )
        }
        Ty::Loader(ty) => {
            let ident = &ty.account_type_path;
            (
                quote! {
                    #ident
                },
                quote! {
                    anchor_lang::Loader
                },
                true,
            )
        }
        Ty::CpiAccount(ty) => {
            let ident = &ty.account_type_path;
            (
                quote! {
                    #ident
                },
                quote! {
                    anchor_lang::CpiAccount
                },
                false,
            )
        }
        Ty::AccountInfo => (
            quote! {
                AccountInfo
            },
            quote! {},
            false,
        ),
        _ => panic!("Invalid type for initializing a program derived address"),
    }
}

pub fn generate_pda(
    f: &Field,
    seeds_constraint: proc_macro2::TokenStream,
    seeds_with_nonce: proc_macro2::TokenStream,
    payer: proc_macro2::TokenStream,
    space: &Option<Expr>,
    assign_nonce: bool,
    kind: &PdaKind,
) -> proc_macro2::TokenStream {
    let field = &f.ident;
    let (account_ty, account_wrapper_ty, is_zero_copy) = parse_ty(f);

    let space = match space {
        // If no explicit space param was given, serialize the type to bytes
        // and take the length (with +8 for the discriminator.)
        None => match is_zero_copy {
            false => {
                quote! {
                    let space = 8 + #account_ty::default().try_to_vec().unwrap().len();
                }
            }
            true => {
                quote! {
                    let space = 8 + anchor_lang::__private::bytemuck::bytes_of(&#account_ty::default()).len();
                }
            }
        },
        // Explicit account size given. Use it.
        Some(s) => quote! {
            let space = #s;
        },
    };

    let nonce_assignment = match assign_nonce {
        false => quote! {},
        true => match &f.ty {
            Ty::CpiAccount(_) => quote! {},
            _ => match is_zero_copy {
                false => quote! {
                    pa.__nonce = nonce;
                },
                // Zero copy is not deserialized, so the data must be lazy loaded.
                true => quote! {
                    pa.load_init()?.__nonce = nonce;
                },
            },
        },
    };

    let (combined_account_ty, try_from) = match f.ty {
        Ty::AccountInfo => (
            quote! {
                AccountInfo
            },
            quote! {
                #field.to_account_info()
            },
        ),
        _ => (
            quote! {
                #account_wrapper_ty<#account_ty>
            },
            quote! {
                #account_wrapper_ty::try_from_init(
                    &#field.to_account_info(),
                )?
            },
        ),
    };

    match kind {
        PdaKind::Token { owner, mint } => quote! {
            let #field: #combined_account_ty = {
                #space
                #payer
                #seeds_constraint

                // Fund the account for rent exemption.
                let required_lamports = rent
                    .minimum_balance(anchor_spl::token::TokenAccount::LEN)
                    .max(1)
                    .saturating_sub(#field.to_account_info().lamports());
                if required_lamports > 0 {
                    anchor_lang::solana_program::program::invoke(
                        &anchor_lang::solana_program::system_instruction::transfer(
                            payer.to_account_info().key,
                            #field.to_account_info().key,
                            required_lamports,
                        ),
                        &[
                            payer.to_account_info(),
                            #field.to_account_info(),
                            system_program.to_account_info().clone(),
                        ],
                    )?;
                }

                // Allocate space.
                anchor_lang::solana_program::program::invoke_signed(
                    &anchor_lang::solana_program::system_instruction::allocate(
                        #field.to_account_info().key,
                        anchor_spl::token::TokenAccount::LEN as u64,
                    ),
                    &[
                        #field.to_account_info(),
                        system_program.clone(),
                    ],
                    &[&#seeds_with_nonce[..]],
                )?;

                // Assign to the spl token program.
                let __ix = anchor_lang::solana_program::system_instruction::assign(
                    #field.to_account_info().key,
                    token_program.to_account_info().key,
                );
                anchor_lang::solana_program::program::invoke_signed(
                    &__ix,
                    &[
                        #field.to_account_info(),
                        system_program.to_account_info(),
                    ],
                    &[&#seeds_with_nonce[..]],
                )?;

                // Initialize the token account.
                let cpi_program = token_program.to_account_info();
                let accounts = anchor_spl::token::InitializeAccount {
                    account: #field.to_account_info(),
                    mint: #mint.to_account_info(),
                    authority: #owner.to_account_info(),
                    rent: rent.to_account_info(),
                };
                let cpi_ctx = CpiContext::new(cpi_program, accounts);
                anchor_spl::token::initialize_account(cpi_ctx)?;
                anchor_lang::CpiAccount::try_from_init(
                    &#field.to_account_info(),
                )?
            };
        },
        PdaKind::Program { owner } => {
            // Owner of the account being created. If not specified,
            // default to the currently executing program.
            let owner = match owner {
                None => quote! {
                    program_id
                },
                Some(o) => quote! {
                    &#o
                },
            };
            quote! {
                let #field = {
                    #space
                    #payer
                    #seeds_constraint

                    let lamports = rent.minimum_balance(space);
                    let ix = anchor_lang::solana_program::system_instruction::create_account(
                        payer.to_account_info().key,
                        #field.to_account_info().key,
                        lamports,
                        space as u64,
                        #owner,
                    );

                    anchor_lang::solana_program::program::invoke_signed(
                        &ix,
                        &[

                            #field.to_account_info(),
                            payer.to_account_info(),
                            system_program.to_account_info(),
                        ],
                        &[&#seeds_with_nonce[..]]
                    ).map_err(|e| {
                        anchor_lang::solana_program::msg!("Unable to create associated account");
                        e
                    })?;

                    // For now, we assume all accounts created with the `associated`
                    // attribute have a `nonce` field in their account.
                    let mut pa: #combined_account_ty = #try_from;

                    #nonce_assignment
                    pa
                };
            }
        }
    }
}

pub fn generate_constraint_associated_seeds(
    f: &Field,
    c: &ConstraintAssociatedGroup,
) -> proc_macro2::TokenStream {
    let field = &f.ident;
    let associated_target = c.associated_target.clone();
    let seeds_no_nonce = if c.associated_seeds.is_empty() {
        quote! {
            &b"anchor"[..],
            #associated_target.to_account_info().key.as_ref(),
        }
    } else {
        let seeds = to_seeds_tts(&c.associated_seeds);
        quote! {
            &b"anchor"[..],
            #associated_target.to_account_info().key.as_ref(),
            #seeds
        }
    };

    let is_find_nonce = match &f.ty {
        Ty::CpiAccount(_) => true,
        Ty::AccountInfo => true,
        _ => c.is_init,
    };
    let associated_field = if is_find_nonce {
        quote! {
            let (__associated_field, nonce) = Pubkey::find_program_address(
                &[#seeds_no_nonce],
                program_id,
            );
        }
    } else {
        let nonce = match &f.ty {
            Ty::ProgramAccount(_) => quote! { #field.__nonce },
            Ty::Loader(_) => {
                // Zero copy is not deserialized, so the data must be lazy loaded.
                quote! { #field.load()?.__nonce }
            }
            _ => panic!("Invalid type for initializing a program derived address"),
        };
        quote! {
            let __associated_field = Pubkey::create_program_address(
                &[#seeds_no_nonce &[#nonce]],
                program_id,
            )?;
        }
    };
    quote! {
        #associated_field
        if &__associated_field != #field.to_account_info().key {
            return Err(anchor_lang::__private::ErrorCode::ConstraintAssociatedInit.into());
        }
    }
}

pub fn generate_constraint_executable(
    f: &Field,
    _c: &ConstraintExecutable,
) -> proc_macro2::TokenStream {
    let name = &f.ident;
    quote! {
        if !#name.to_account_info().executable {
            return Err(anchor_lang::__private::ErrorCode::ConstraintExecutable.into());
        }
    }
}

pub fn generate_constraint_state(f: &Field, c: &ConstraintState) -> proc_macro2::TokenStream {
    let program_target = c.program_target.clone();
    let ident = &f.ident;
    let account_ty = match &f.ty {
        Ty::CpiState(ty) => &ty.account_type_path,
        _ => panic!("Invalid state constraint"),
    };
    quote! {
        // Checks the given state account is the canonical state account for
        // the target program.
        if #ident.to_account_info().key != &anchor_lang::CpiState::<#account_ty>::address(#program_target.to_account_info().key) {
            return Err(anchor_lang::__private::ErrorCode::ConstraintState.into());
        }
        if #ident.to_account_info().owner != #program_target.to_account_info().key {
            return Err(anchor_lang::__private::ErrorCode::ConstraintState.into());
        }
    }
}

// Returns the inner part of the seeds slice as a token stream.
fn to_seeds_tts(seeds: &[syn::Expr]) -> proc_macro2::TokenStream {
    assert!(!seeds.is_empty());
    let seed_0 = &seeds[0];
    let mut tts = match seed_0 {
        syn::Expr::Path(_) => quote! {
            anchor_lang::Key::key(&#seed_0).as_ref(),
        },
        _ => quote! {
            #seed_0,
        },
    };
    for seed in &seeds[1..] {
        tts = match seed {
            syn::Expr::Path(_) => quote! {
                #tts
                anchor_lang::Key::key(&#seed).as_ref(),
            },
            _ => quote! {
                #tts
                #seed,
            },
        };
    }
    tts
}
