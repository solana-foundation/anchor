//! Instruments each line of user code with `sol_log_compute_units`

use quote::ToTokens;
use syn::{ItemMod, Stmt};

pub fn generate(mut module: ItemMod) -> ItemMod {
    let Some((_, content)) = module.content.as_mut() else {
        return module;
    };
    // Insert a message and CU log after each statement
    for insn in content.iter_mut().filter_map(|item| match item {
        syn::Item::Fn(item_fn) => Some(item_fn),
        _ => None,
    }) {
        let stmts = std::mem::take(&mut insn.block.stmts);
        let interpsersed: Vec<Stmt> = stmts
            .into_iter()
            .flat_map(|stmt| match stmt {
                // Last expression of a block - don't add code after
                Stmt::Expr(_) => vec![stmt],
                _ => {
                    let as_str = stmt.to_token_stream().to_string();
                    let log: Stmt = syn::parse_str(&format!(
                        r#"{{
    ::anchor_lang::solana_program::log::sol_log(concat!("Executing `", stringify!({as_str}), "`"));
    ::anchor_lang::solana_program::log::sol_log_compute_units();
                    }};"#,
                    ))
                    .unwrap();
                    vec![stmt, log]
                }
            })
            .collect();
        insn.block.stmts = interpsersed;
    }
    module
}
