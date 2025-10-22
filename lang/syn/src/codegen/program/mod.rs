use crate::parser::reload_check;
use crate::Program;
use quote::quote;

mod accounts;
pub mod common;
mod cpi;
mod dispatch;
mod entry;
mod handlers;
mod idl;
mod instruction;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let mod_name = &program.name;

    // Check for missing reload() after CPI
    let warnings = reload_check::check_program(program);
    let reload_warnings = if !warnings.is_empty() {
        let warning_msgs: Vec<_> = warnings
            .iter()
            .map(|w| {
                quote! {
                    const _: () = {
                        const WARNING: &str = concat!(
                            "\n",
                            "⚠️  Potential missing reload() after CPI:\n",
                            "   ", #w, "\n",
                            "   Call .reload()? on accounts after CPI and before accessing their data.\n"
                        );
                        // Force the warning to be visible by using it
                        let _ = WARNING;
                        // Use a type that will show the warning in the compile output
                        #[deprecated(note = "See warning above")]
                        const _RELOAD_WARNING: () = ();
                        let _ = _RELOAD_WARNING;
                    };
                }
            })
            .collect();
        quote! { #(#warning_msgs)* }
    } else {
        quote! {}
    };

    let entry = entry::generate(program);
    let dispatch = dispatch::generate(program);
    let handlers = handlers::generate(program);
    let user_defined_program = &program.program_mod;
    let instruction = instruction::generate(program);
    let cpi = cpi::generate(program);
    let accounts = accounts::generate(program);

    #[allow(clippy::let_and_return)]
    let ret = {
        quote! {
            // TODO: remove once we allow segmented paths in `Accounts` structs.
            use self::#mod_name::*;

            // Emit reload warnings at compile time
            #reload_warnings

            #entry
            #dispatch
            #handlers
            #user_defined_program
            #instruction
            #cpi
            #accounts
        }
    };

    #[cfg(feature = "idl-build")]
    {
        let idl_build_impl = crate::idl::gen_idl_print_fn_program(program);
        return quote! {
            #ret
            #idl_build_impl
        };
    };

    #[allow(unreachable_code)]
    ret
}
