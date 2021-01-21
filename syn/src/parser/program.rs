use crate::{Program, Rpc, RpcArg};

pub fn parse(program_mod: syn::ItemMod) -> Program {
    let mod_ident = &program_mod.ident;
    let methods: Vec<&syn::ItemFn> = program_mod
        .content
        .as_ref()
        .unwrap()
        .1
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(item_fn) => Some(item_fn),
            _ => None,
        })
        .collect();

    let rpcs: Vec<Rpc> = methods
        .clone()
        .into_iter()
        .map(|method: &syn::ItemFn| {
            let mut args: Vec<RpcArg> = method
                .sig
                .inputs
                .iter()
                .map(|arg: &syn::FnArg| match arg {
                    syn::FnArg::Typed(arg) => {
                        let ident = match &*arg.pat {
                            syn::Pat::Ident(ident) => &ident.ident,
                            _ => panic!("invalid syntax"),
                        };
                        RpcArg {
                            name: ident.clone(),
                            raw_arg: arg.clone(),
                        }
                    }
                    _ => panic!("invalid syntax"),
                })
                .collect();
            // Remove the Anchor accounts argument
            let anchor = args.remove(0);
            let anchor_ident = extract_ident(&anchor.raw_arg).clone();

            Rpc {
                raw_method: method.clone(),
                ident: method.sig.ident.clone(),
                args,
                anchor_ident,
            }
        })
        .collect();

    Program {
        rpcs,
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
