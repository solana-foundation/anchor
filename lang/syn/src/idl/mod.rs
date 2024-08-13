#![allow(dead_code)]

mod accounts;
mod address;
mod common;
mod constant;
mod defined;
mod error;
mod event;
mod external;
mod program;

pub use accounts::gen_idl_build_impl_accounts_struct;
pub use address::gen_idl_print_fn_address;
pub use constant::gen_idl_print_fn_constant;
pub use defined::{impl_idl_build_enum, impl_idl_build_struct, impl_idl_build_union};
pub use error::gen_idl_print_fn_error;
pub use event::gen_idl_print_fn_event;
pub use program::gen_idl_print_fn_program;
