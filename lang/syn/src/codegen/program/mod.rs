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
#[cfg(feature = "instrument-compute-units")]
mod instrument;

pub fn generate(program: &Program) -> proc_macro2::TokenStream {
    let mod_name = &program.name;

    let entry = entry::generate(program);
    let dispatch = dispatch::generate(program);
    let handlers = handlers::generate(program);
    #[cfg(not(feature = "instrument-compute-units"))]
    let user_defined_program = &program.program_mod;
    #[cfg(feature = "instrument-compute-units")]
    let user_defined_program = &instrument::generate(program.program_mod.clone());

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
        let idl_build_impl = crate::idl::gen_idl_print_fn_program(program);
        return quote! {
            #ret
            #idl_build_impl
        };
    };

    #[allow(unreachable_code)]
    ret
}
