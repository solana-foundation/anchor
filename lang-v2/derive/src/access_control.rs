use {
    proc_macro::TokenStream, proc_macro2::TokenStream as TokenStream2, quote::quote,
    syn::parse_macro_input,
};

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut args = args.to_string();
    args.retain(|c| !c.is_whitespace());
    let access_control: Vec<TokenStream2> = args
        .split(')')
        .filter(|ac| !ac.is_empty())
        .map(|ac| format!("{ac})?;"))
        .map(|ac| ac.parse().unwrap())
        .collect();

    let item_fn = parse_macro_input!(input as syn::ItemFn);
    let fn_attrs = item_fn.attrs;
    let fn_vis = item_fn.vis;
    let fn_sig = item_fn.sig;
    let fn_stmts = item_fn.block.stmts;

    TokenStream::from(quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            #(#access_control)*
            #(#fn_stmts)*
        }
    })
}
