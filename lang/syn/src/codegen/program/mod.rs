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
        let no_docs = crate::idl::build::get_no_docs();
        let idl_build = crate::idl::build::gen_idl_print_function_for_program(program, no_docs);

        return quote! {
            #ret
            #idl_build
        };
    };

    #[allow(unreachable_code)]
    ret
}
