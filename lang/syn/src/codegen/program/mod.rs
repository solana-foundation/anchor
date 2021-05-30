use crate::Program;
use quote::quote;

mod accounts;
pub mod common;
mod cpi;
mod dispatch;
mod entry;
mod handlers;
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

    quote! {
        // TODO: remove once we allow segmented paths in `Accounts` structs.
        use #mod_name::*;

        #entry
        #dispatch
        #handlers
        #user_defined_program
        #instruction
        #cpi
        #accounts
    }
}
