use crate::parser;
use crate::{Ix, IxArg, Program, State, StateInterface, StateIx};

// Name of the attribute denoting a state struct.
const STATE_STRUCT_ATTRIBUTE: &str = "state";

// Reserved keyword for the constructor method.
const CTOR_METHOD_NAME: &str = "new";

pub fn parse(program_mod: syn::ItemMod) -> Program {
    let mod_ident = &program_mod.ident;
    let mod_content = &program_mod.content.as_ref().unwrap().1;

    // Parse program state.
    let state: Option<State> = {
        // Parse `struct` marked with the `#[state]` attribute.
        let strct: Option<(&syn::ItemStruct, bool)> = mod_content
            .iter()
            .filter_map(|item| match item {
                syn::Item::Struct(item_strct) => {
                    let attrs = &item_strct.attrs;
                    if attrs.is_empty() {
                        return None;
                    }
                    let attr_label = attrs[0].path.get_ident().map(|i| i.to_string());
                    if attr_label != Some(STATE_STRUCT_ATTRIBUTE.to_string()) {
                        return None;
                    }
                    let is_zero_copy = parser::tts_to_string(&attrs[0].tokens) == "(zero_copy)";
                    Some((item_strct, is_zero_copy))
                }
                _ => None,
            })
            .next();
        // Parse `impl` block for the state struct.
        let impl_block: Option<syn::ItemImpl> = match strct {
            None => None,
            Some((strct, _)) => mod_content
                .iter()
                .filter_map(|item| match item {
                    syn::Item::Impl(item_impl) => {
                        let impl_ty_str = parser::tts_to_string(&item_impl.self_ty);
                        let strct_name = strct.ident.to_string();
                        if item_impl.trait_.is_some() {
                            return None;
                        }
                        if strct_name != impl_ty_str {
                            return None;
                        }
                        Some(item_impl.clone())
                    }
                    _ => None,
                })
                .next(),
        };
        // Parse ctor and the generic type in `Context<MY-TYPE>`.
        let ctor_and_anchor = match &impl_block {
            None => None,
            Some(impl_block) => {
                impl_block
                    .items
                    .iter()
                    .filter_map(|item: &syn::ImplItem| match item {
                        syn::ImplItem::Method(m) => {
                            if m.sig.ident == CTOR_METHOD_NAME {
                                let (_, is_zero_copy) = strct.as_ref().unwrap();
                                let ctx_arg = {
                                    if *is_zero_copy {
                                        // Second param is context.
                                        let mut iter = m.sig.inputs.iter();
                                        iter.next().expect("First param must be receiver");
                                        iter.next().expect("Second param must be Context")
                                    } else {
                                        // First param is ctx.
                                        m.sig.inputs.first().unwrap()
                                    }
                                };
                                match ctx_arg {
                                    syn::FnArg::Receiver(_) => panic!("invalid ctor syntax"),
                                    syn::FnArg::Typed(arg) => {
                                        Some((m.clone(), extract_ident(&arg).clone()))
                                    }
                                }
                            } else {
                                None
                            }
                        }
                        _ => None,
                    })
                    .next()
            }
        };
        // Parse all methods in the above `impl` block.
        let methods: Option<Vec<StateIx>> = impl_block.as_ref().map(|impl_block| {
            impl_block
                .items
                .iter()
                .filter(|item| match item {
                    syn::ImplItem::Method(m) => m.sig.ident.to_string() != CTOR_METHOD_NAME,
                    _ => false,
                })
                .filter_map(|item: &syn::ImplItem| match item {
                    syn::ImplItem::Method(m) => match m.sig.inputs.first() {
                        None => None,
                        Some(arg) => match arg {
                            syn::FnArg::Typed(_) => None,
                            syn::FnArg::Receiver(_) => {
                                let mut args = m
                                    .sig
                                    .inputs
                                    .iter()
                                    .filter_map(|arg| match arg {
                                        syn::FnArg::Receiver(_) => None,
                                        syn::FnArg::Typed(arg) => Some(arg),
                                    })
                                    .map(|raw_arg| {
                                        let ident = match &*raw_arg.pat {
                                            syn::Pat::Ident(ident) => &ident.ident,
                                            _ => panic!("invalid syntax"),
                                        };
                                        IxArg {
                                            name: ident.clone(),
                                            raw_arg: raw_arg.clone(),
                                        }
                                    })
                                    .collect::<Vec<IxArg>>();
                                // Remove the Anchor accounts argument
                                let anchor = args.remove(0);
                                let anchor_ident = extract_ident(&anchor.raw_arg).clone();

                                Some(StateIx {
                                    raw_method: m.clone(),
                                    ident: m.sig.ident.clone(),
                                    args,
                                    anchor_ident,
                                    has_receiver: true,
                                })
                            }
                        },
                    },
                    _ => None,
                })
                .collect()
        });
        // Parse all trait implementations for the above `#[state]` struct.
        let trait_impls: Option<Vec<StateInterface>> = strct.map(|_strct| {
            mod_content
                .iter()
                .filter_map(|item| match item {
                    syn::Item::Impl(item_impl) => {
                        let trait_name = match &item_impl.trait_ {
                            None => return None,
                            Some((_, path, _)) => path
                                .segments
                                .iter()
                                .next()
                                .expect("Must have one segment in a path")
                                .ident
                                .clone()
                                .to_string(),
                        };
                        let methods = item_impl
                            .items
                            .iter()
                            .filter_map(|item: &syn::ImplItem| match item {
                                syn::ImplItem::Method(m) => match m.sig.inputs.first() {
                                    None => None,
                                    Some(_arg) => {
                                        let mut has_receiver = false;
                                        let mut args = m
                                            .sig
                                            .inputs
                                            .iter()
                                            .filter_map(|arg| match arg {
                                                syn::FnArg::Receiver(_) => {
                                                    has_receiver = true;
                                                    None
                                                }
                                                syn::FnArg::Typed(arg) => Some(arg),
                                            })
                                            .map(|raw_arg| {
                                                let ident = match &*raw_arg.pat {
                                                    syn::Pat::Ident(ident) => &ident.ident,
                                                    _ => panic!("invalid syntax"),
                                                };
                                                IxArg {
                                                    name: ident.clone(),
                                                    raw_arg: raw_arg.clone(),
                                                }
                                            })
                                            .collect::<Vec<IxArg>>();
                                        // Remove the Anchor accounts argument
                                        let anchor = args.remove(0);
                                        let anchor_ident = extract_ident(&anchor.raw_arg).clone();

                                        Some(StateIx {
                                            raw_method: m.clone(),
                                            ident: m.sig.ident.clone(),
                                            args,
                                            anchor_ident,
                                            has_receiver,
                                        })
                                    }
                                },
                                _ => None,
                            })
                            .collect();
                        Some(StateInterface {
                            trait_name,
                            methods,
                        })
                    }
                    _ => None,
                })
                .collect::<Vec<StateInterface>>()
        });
        // Put it all together.
        strct.map(|(strct, is_zero_copy)| {
            // Chop off the `#[state]` attribute. It's just a marker.
            let mut strct = strct.clone();
            strct.attrs = vec![];

            State {
                name: strct.ident.to_string(),
                strct,
                interfaces: trait_impls,
                impl_block_and_methods: impl_block.map(|impl_block| (impl_block, methods.unwrap())),
                ctor_and_anchor,
                is_zero_copy,
            }
        })
    };
    // Parse all non-state ix handlers.
    let ixs: Vec<Ix> = mod_content
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(item_fn) => Some(item_fn),
            _ => None,
        })
        .map(|method: &syn::ItemFn| {
            let mut args: Vec<IxArg> = method
                .sig
                .inputs
                .iter()
                .map(|arg: &syn::FnArg| match arg {
                    syn::FnArg::Typed(arg) => {
                        let ident = match &*arg.pat {
                            syn::Pat::Ident(ident) => &ident.ident,
                            _ => panic!("invalid syntax"),
                        };
                        IxArg {
                            name: ident.clone(),
                            raw_arg: arg.clone(),
                        }
                    }
                    _ => panic!("invalid syntax"),
                })
                .collect();
            // Remove the Context argument
            let anchor = args.remove(0);
            let anchor_ident = extract_ident(&anchor.raw_arg).clone();

            Ix {
                raw_method: method.clone(),
                ident: method.sig.ident.clone(),
                args,
                anchor_ident,
            }
        })
        .collect();

    Program {
        state,
        ixs,
        name: mod_ident.clone(),
        program_mod,
    }
}

fn extract_ident(path_ty: &syn::PatType) -> &proc_macro2::Ident {
    let p = match &*path_ty.ty {
        syn::Type::Path(p) => &p.path,
        _ => panic!("invalid syntax"),
    };
    let segment = p.segments.first().unwrap();
    let generic_args = match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => args,
        _ => panic!("invalid syntax"),
    };
    let generic_ty = generic_args
        .args
        .iter()
        .filter_map(|arg| match arg {
            syn::GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .next()
        .unwrap();
    let path = match generic_ty {
        syn::Type::Path(ty_path) => &ty_path.path,
        _ => panic!("invalid syntax"),
    };
    &path.segments[0].ident
}
