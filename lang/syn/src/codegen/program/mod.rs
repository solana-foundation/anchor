use crate::Program;
use quote::quote_spanned;
use syn::spanned::Spanned;

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
    let program_span = program.program_mod.span();

    let entry = entry::generate(program);
    let dispatch = dispatch::generate(program);
    let handlers = handlers::generate(program);
    let user_defined_program = &program.program_mod;
    let instruction = instruction::generate(program);
    let cpi = cpi::generate(program);
    let accounts = accounts::generate(program);

    #[allow(clippy::let_and_return)]
    let ret = {
        quote_spanned! { program_span =>
            // TODO: remove once we allow segmented paths in `Accounts` structs.
            use self::#mod_name::*;

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
        return quote_spanned! { program_span =>
            #ret
            #idl_build_impl
        };
    };

    #[allow(unreachable_code)]
    ret
}
