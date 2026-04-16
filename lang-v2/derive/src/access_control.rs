use {
    proc_macro::TokenStream,
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    syn::{parse::Parser, parse_macro_input, punctuated::Punctuated, Expr, Token},
};

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
    let exprs = match parser.parse(args) {
        Ok(exprs) => exprs,
        Err(err) => return err.to_compile_error().into(),
    };
    let access_control: Vec<TokenStream2> = exprs
        .into_iter()
        .map(|expr| quote! { #expr?; })
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
