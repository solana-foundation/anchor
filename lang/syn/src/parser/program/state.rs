use crate::parser;
use crate::parser::docs;
use crate::parser::program::ctx_accounts_ident;
use crate::{IxArg, State, StateInterface, StateIx};
use syn::parse::{Error as ParseError, Result as ParseResult};
use syn::spanned::Spanned;

// Name of the attribute denoting a state struct.
const STATE_STRUCT_ATTRIBUTE: &str = "state";

// Reserved keyword for the constructor method.
const CTOR_METHOD_NAME: &str = "new";

// Parse the state from the program mod definition.
pub fn parse(program_mod: &syn::ItemMod) -> ParseResult<Option<State>> {
    let mod_content = &program_mod
        .content
        .as_ref()
        .ok_or_else(|| ParseError::new(program_mod.span(), "program content not provided"))?
        .1;

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
    let ctor_and_anchor: Option<(syn::ImplItemMethod, syn::Ident)> = impl_block
        .as_ref()
        .map(|impl_block| {
            let r: Option<ParseResult<_>> = impl_block
                .items
                .iter()
                .filter_map(|item: &syn::ImplItem| match item {
                    syn::ImplItem::Method(m) => match m.sig.ident == CTOR_METHOD_NAME {
                        false => None,
                        true => Some(m),
                    },
                    _ => None,
                })
                .map(|m: &syn::ImplItemMethod| {
                    let (_, is_zero_copy) = strct
                        .as_ref()
                        .expect("impl_block exists therefore the struct exists");
                    let ctx_arg = {
                        if *is_zero_copy {
                            // Second param is context.
                            let mut iter = m.sig.inputs.iter();
                            match iter.next() {
                                None => {
                                    return Err(ParseError::new(
                                        m.sig.span(),
                                        "first parameter must be &mut self",
                                    ))
                                }
                                Some(arg) => match arg {
                                    syn::FnArg::Receiver(r) => {
                                        if r.mutability.is_none() {
                                            return Err(ParseError::new(
                                                m.sig.span(),
                                                "first parameter must be &mut self",
                                            ));
                                        }
                                    }
                                    syn::FnArg::Typed(_) => {
                                        return Err(ParseError::new(
                                            m.sig.span(),
                                            "first parameter must be &mut self",
                                        ))
                                    }
                                },
                            };
                            match iter.next() {
                                None => {
                                    return Err(ParseError::new(
                                        m.sig.span(),
                                        "second parameter must be the Context",
                                    ))
                                }
                                Some(ctx_arg) => match ctx_arg {
                                    syn::FnArg::Receiver(_) => {
                                        return Err(ParseError::new(
                                            ctx_arg.span(),
                                            "second parameter must be the Context",
                                        ))
                                    }
                                    syn::FnArg::Typed(arg) => arg,
                                },
                            }
                        } else {
                            match m.sig.inputs.first() {
                                None => {
                                    return Err(ParseError::new(
                                        m.sig.span(),
                                        "first parameter must be the Context",
                                    ))
                                }
                                Some(ctx_arg) => match ctx_arg {
                                    syn::FnArg::Receiver(_) => {
                                        return Err(ParseError::new(
                                            ctx_arg.span(),
                                            "second parameter must be the Context",
                                        ))
                                    }
                                    syn::FnArg::Typed(arg) => arg,
                                },
                            }
                        }
                    };
                    Ok((m.clone(), ctx_accounts_ident(ctx_arg)?))
                })
                .next();
            r.transpose()
        })
        .transpose()?
        .unwrap_or(None);

    // Parse all methods in the above `impl` block.
    let methods: Option<Vec<StateIx>> = impl_block
        .as_ref()
        .map(|impl_block| {
            impl_block
                .items
                .iter()
                .filter_map(|item| match item {
                    syn::ImplItem::Method(m) => match m.sig.ident != CTOR_METHOD_NAME {
                        false => None,
                        true => Some(m),
                    },
                    _ => None,
                })
                .map(|m: &syn::ImplItemMethod| {
                    let mut args = m
                        .sig
                        .inputs
                        .iter()
                        .filter_map(|arg| match arg {
                            syn::FnArg::Receiver(_) => None,
                            syn::FnArg::Typed(arg) => Some(arg),
                        })
                        .map(|raw_arg| {
                            let docs = docs::parse(&raw_arg.attrs);
                            let ident = match &*raw_arg.pat {
                                syn::Pat::Ident(ident) => &ident.ident,
                                _ => {
                                    return Err(ParseError::new(
                                        raw_arg.pat.span(),
                                        "unexpected type argument",
                                    ))
                                }
                            };
                            Ok(IxArg {
                                name: ident.clone(),
                                docs,
                                raw_arg: raw_arg.clone(),
                            })
                        })
                        .collect::<ParseResult<Vec<IxArg>>>()?;
                    // Remove the Anchor accounts argument
                    let anchor = args.remove(0);
                    let anchor_ident = ctx_accounts_ident(&anchor.raw_arg)?;

                    Ok(StateIx {
                        raw_method: m.clone(),
                        ident: m.sig.ident.clone(),
                        args,
                        anchor_ident,
                        has_receiver: true,
                    })
                })
                .collect::<ParseResult<Vec<_>>>()
        })
        .transpose()?;

    // Parse all trait implementations for the above `#[state]` struct.
    let trait_impls: Option<Vec<StateInterface>> = strct
        .map(|_strct| {
            mod_content
                .iter()
                .filter_map(|item| match item {
                    syn::Item::Impl(item_impl) => match &item_impl.trait_ {
                        None => None,
                        Some((_, path, _)) => {
                            let trait_name = path
                                .segments
                                .iter()
                                .next()
                                .expect("Must have one segment in a path")
                                .ident
                                .clone()
                                .to_string();
                            Some((item_impl, trait_name))
                        }
                    },
                    _ => None,
                })
                .map(|(item_impl, trait_name)| {
                    let methods = item_impl
                        .items
                        .iter()
                        .filter_map(|item: &syn::ImplItem| match item {
                            syn::ImplItem::Method(m) => Some(m),
                            _ => None,
                        })
                        .map(|m: &syn::ImplItemMethod| {
                            match m.sig.inputs.first() {
                                None => Err(ParseError::new(
                                    m.sig.inputs.span(),
                                    "state methods must have a self argument",
                                )),
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
                                            let docs = docs::parse(&raw_arg.attrs);
                                            let ident = match &*raw_arg.pat {
                                                syn::Pat::Ident(ident) => &ident.ident,
                                                _ => panic!("invalid syntax"),
                                            };
                                            IxArg {
                                                name: ident.clone(),
                                                docs,
                                                raw_arg: raw_arg.clone(),
                                            }
                                        })
                                        .collect::<Vec<IxArg>>();
                                    // Remove the Anchor accounts argument
                                    let anchor = args.remove(0);
                                    let anchor_ident = ctx_accounts_ident(&anchor.raw_arg)?;

                                    Ok(StateIx {
                                        raw_method: m.clone(),
                                        ident: m.sig.ident.clone(),
                                        args,
                                        anchor_ident,
                                        has_receiver,
                                    })
                                }
                            }
                        })
                        .collect::<ParseResult<Vec<StateIx>>>()?;
                    Ok(StateInterface {
                        trait_name,
                        methods,
                    })
                })
                .collect::<ParseResult<Vec<StateInterface>>>()
        })
        .transpose()?;

    Ok(strct.map(|(strct, is_zero_copy)| {
        // Chop off the `#[state]` attribute. It's just a marker.
        //
        // TODO: instead of mutating the syntax, we should just implement
        //       a macro that does nothing.
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
    }))
}
