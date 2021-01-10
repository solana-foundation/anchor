//! DSL syntax tokens.

pub mod codegen;
#[cfg(feature = "idl")]
pub mod idl;
pub mod parser;

#[derive(Debug)]
pub struct Program {
    pub rpcs: Vec<Rpc>,
    pub name: syn::Ident,
    pub program_mod: syn::ItemMod,
}

#[derive(Debug)]
pub struct Rpc {
    pub raw_method: syn::ItemFn,
    pub ident: syn::Ident,
    pub args: Vec<RpcArg>,
    pub anchor_ident: syn::Ident,
}

#[derive(Debug)]
pub struct RpcArg {
    pub name: proc_macro2::Ident,
    pub raw_arg: syn::PatType,
}

pub struct AccountsStruct {
    // Name of the accounts struct.
    pub ident: syn::Ident,
    // Generics + lifetimes on the accounts struct.
    pub generics: syn::Generics,
    // Fields on the accounts struct.
    pub fields: Vec<Field>,
}

impl AccountsStruct {
    pub fn new(strct: syn::ItemStruct, fields: Vec<Field>) -> Self {
        let ident = strct.ident.clone();
        let generics = strct.generics.clone();
        Self {
            ident,
            generics,
            fields,
        }
    }

    pub fn account_tys(&self) -> Vec<String> {
        self.fields
            .iter()
            .filter_map(|f| match &f.ty {
                Ty::ProgramAccount(pty) => Some(pty.account_ident.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>()
    }
}

// An account in the accounts struct.
pub struct Field {
    pub ident: syn::Ident,
    pub ty: Ty,
    pub constraints: Vec<Constraint>,
    pub is_mut: bool,
    pub is_signer: bool,
    pub is_init: bool,
}

// A type of an account field.
#[derive(PartialEq)]
pub enum Ty {
    AccountInfo,
    ProgramAccount(ProgramAccountTy),
}

#[derive(PartialEq)]
pub struct ProgramAccountTy {
    // The struct type of the account.
    pub account_ident: syn::Ident,
}

// An access control constraint for an account.
pub enum Constraint {
    Signer(ConstraintSigner),
    BelongsTo(ConstraintBelongsTo),
    Literal(ConstraintLiteral),
    Owner(ConstraintOwner),
}

pub struct ConstraintBelongsTo {
    pub join_target: proc_macro2::Ident,
}

pub struct ConstraintSigner {}

pub struct ConstraintLiteral {
    pub tokens: proc_macro2::TokenStream,
}

pub enum ConstraintOwner {
    Program,
    Skip,
}
