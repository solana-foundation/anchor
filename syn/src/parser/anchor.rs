use crate::{
    AccountsStruct, Constraint, ConstraintBelongsTo, ConstraintLiteral, ConstraintOwner,
    ConstraintSigner, Field, ProgramAccountTy, Ty,
};
use quote::quote;

pub fn parse(strct: &syn::ItemStruct) -> AccountsStruct {
    let fields = match &strct.fields {
        syn::Fields::Named(fields) => fields,
        _ => panic!("invalid input"),
    };

    let fields: Vec<Field> = fields
        .named
        .iter()
        .map(|f: &syn::Field| {
            let anchor_attr = {
                let anchor_attrs: Vec<&syn::Attribute> = f
                    .attrs
                    .iter()
                    .filter_map(|attr: &syn::Attribute| {
                        if attr.path.segments.len() != 1 {
                            return None;
                        }
                        if attr.path.segments[0].ident.to_string() != "account" {
                            return None;
                        }
                        Some(attr)
                    })
                    .collect();
                assert!(anchor_attrs.len() == 1);
                anchor_attrs[0]
            };
            parse_field(f, anchor_attr)
        })
        .collect();

    AccountsStruct::new(strct.clone(), fields)
}

// Parses an inert #[anchor] attribute specifying the DSL.
fn parse_field(f: &syn::Field, anchor: &syn::Attribute) -> Field {
    let ident = f.ident.clone().unwrap();
    let ty = parse_ty(f);
    let (constraints, is_mut, is_signer) = parse_constraints(anchor, &ty);
    Field {
        ident,
        ty,
        constraints,
        is_mut,
        is_signer,
    }
}

fn parse_ty(f: &syn::Field) -> Ty {
    let path = match &f.ty {
        syn::Type::Path(ty_path) => ty_path.path.clone(),
        _ => panic!("invalid type"),
    };
    // TODO: allow segmented paths.
    assert!(path.segments.len() == 1);
    let segments = &path.segments[0];
    match segments.ident.to_string().as_str() {
        "ProgramAccount" => Ty::ProgramAccount(parse_program_account(&path)),
        "AccountInfo" => Ty::AccountInfo,
        _ => panic!("invalid type"),
    }
}

fn parse_program_account(path: &syn::Path) -> ProgramAccountTy {
    let segments = &path.segments[0];
    let account_ident = match &segments.arguments {
        syn::PathArguments::AngleBracketed(args) => {
            // Expected: <'info, MyType>.
            assert!(args.args.len() == 2);
            match &args.args[1] {
                syn::GenericArgument::Type(ty) => match ty {
                    syn::Type::Path(ty_path) => {
                        // TODO: allow segmented paths.
                        assert!(ty_path.path.segments.len() == 1);
                        let path_segment = &ty_path.path.segments[0];
                        path_segment.ident.clone()
                    }
                    _ => panic!("Invalid ProgramAccount"),
                },
                _ => panic!("Invalid ProgramAccount"),
            }
        }
        _ => panic!("Invalid ProgramAccount"),
    };
    ProgramAccountTy { account_ident }
}

fn parse_constraints(anchor: &syn::Attribute, ty: &Ty) -> (Vec<Constraint>, bool, bool) {
    let mut tts = anchor.tokens.clone().into_iter();
    let g_stream = match tts.next().expect("Must have a token group") {
        proc_macro2::TokenTree::Group(g) => g.stream(),
        _ => panic!("Invalid syntax"),
    };

    let mut is_mut = false;
    let mut is_signer = false;
    let mut constraints = vec![];
    let mut has_owner_constraint = false;

    let mut inner_tts = g_stream.into_iter();
    while let Some(token) = inner_tts.next() {
        match token {
            proc_macro2::TokenTree::Ident(ident) => match ident.to_string().as_str() {
                "mut" => {
                    is_mut = true;
                }
                "signer" => {
                    is_signer = true;
                    constraints.push(Constraint::Signer(ConstraintSigner {}));
                }
                "belongs_to" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let join_target = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::BelongsTo(ConstraintBelongsTo { join_target }))
                }
                "owner" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let owner = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    let constraint = match owner.to_string().as_str() {
                        "program" => ConstraintOwner::Program,
                        "skip" => ConstraintOwner::Skip,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::Owner(constraint));
                    has_owner_constraint = true;
                }
                _ => {
                    panic!("invalid syntax");
                }
            },
            proc_macro2::TokenTree::Punct(punct) => {
                if punct.as_char() != ',' {
                    panic!("invalid syntax");
                }
            }
            proc_macro2::TokenTree::Literal(literal) => {
                let tokens: proc_macro2::TokenStream =
                    literal.to_string().replace("\"", "").parse().unwrap();
                constraints.push(Constraint::Literal(ConstraintLiteral { tokens }));
            }
            _ => {
                panic!("invalid syntax");
            }
        }
    }

    if !has_owner_constraint {
        if ty == &Ty::AccountInfo {
            constraints.push(Constraint::Owner(ConstraintOwner::Skip));
        } else {
            constraints.push(Constraint::Owner(ConstraintOwner::Program));
        }
    }

    (constraints, is_mut, is_signer)
}
