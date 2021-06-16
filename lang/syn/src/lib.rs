use codegen::accounts as accounts_codegen;
use codegen::program as program_codegen;
use parser::accounts as accounts_parser;
use parser::program as program_parser;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::ops::Deref;
use syn::ext::IdentExt;
use syn::parse::{Error as ParseError, Parse, ParseStream, Result as ParseResult};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{
    Expr, Generics, Ident, ImplItemMethod, ItemEnum, ItemFn, ItemImpl, ItemMod, ItemStruct, LitInt,
    LitStr, PatType, Token,
};

pub mod codegen;
#[cfg(feature = "hash")]
pub mod hash;
#[cfg(not(feature = "hash"))]
pub(crate) mod hash;
#[cfg(feature = "idl")]
pub mod idl;
pub mod parser;

#[derive(Debug)]
pub struct Program {
    pub state: Option<State>,
    pub ixs: Vec<Ix>,
    pub name: Ident,
    pub program_mod: ItemMod,
}

impl Parse for Program {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let program_mod = <ItemMod as Parse>::parse(input)?;
        program_parser::parse(program_mod)
    }
}

impl From<&Program> for TokenStream {
    fn from(program: &Program) -> Self {
        program_codegen::generate(program)
    }
}

impl ToTokens for Program {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend::<TokenStream>(self.into());
    }
}

#[derive(Debug)]
pub struct State {
    pub name: String,
    pub strct: ItemStruct,
    pub ctor_and_anchor: Option<(ImplItemMethod, Ident)>,
    pub impl_block_and_methods: Option<(ItemImpl, Vec<StateIx>)>,
    pub interfaces: Option<Vec<StateInterface>>,
    pub is_zero_copy: bool,
}

#[derive(Debug)]
pub struct StateIx {
    pub raw_method: ImplItemMethod,
    pub ident: Ident,
    pub args: Vec<IxArg>,
    pub anchor_ident: Ident,
    // True if there exists a &self on the method.
    pub has_receiver: bool,
}

#[derive(Debug)]
pub struct StateInterface {
    pub trait_name: String,
    pub methods: Vec<StateIx>,
}

#[derive(Debug)]
pub struct Ix {
    pub raw_method: ItemFn,
    pub ident: Ident,
    pub args: Vec<IxArg>,
    // The ident for the struct deriving Accounts.
    pub anchor_ident: Ident,
}

#[derive(Debug)]
pub struct IxArg {
    pub name: Ident,
    pub raw_arg: PatType,
}

#[derive(Debug)]
pub struct AccountsStruct {
    // Name of the accounts struct.
    pub ident: Ident,
    // Generics + lifetimes on the accounts struct.
    pub generics: Generics,
    // Fields on the accounts struct.
    pub fields: Vec<AccountField>,
    // Instruction data api expression.
    instruction_api: Option<Punctuated<Expr, Comma>>,
}

impl Parse for AccountsStruct {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let strct = <ItemStruct as Parse>::parse(input)?;
        accounts_parser::parse(&strct)
    }
}

impl From<&AccountsStruct> for TokenStream {
    fn from(accounts: &AccountsStruct) -> Self {
        accounts_codegen::generate(accounts)
    }
}

impl ToTokens for AccountsStruct {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend::<TokenStream>(self.into());
    }
}

impl AccountsStruct {
    pub fn new(
        strct: ItemStruct,
        fields: Vec<AccountField>,
        instruction_api: Option<Punctuated<Expr, Comma>>,
    ) -> Self {
        let ident = strct.ident.clone();
        let generics = strct.generics;
        Self {
            ident,
            generics,
            fields,
            instruction_api,
        }
    }
}

#[derive(Debug)]
pub enum AccountField {
    Field(Field),
    CompositeField(CompositeField),
}

#[derive(Debug)]
pub struct Field {
    pub ident: Ident,
    pub constraints: ConstraintGroup,
    pub instruction_constraints: ConstraintGroup,
    pub ty: Ty,
}

#[derive(Debug)]
pub struct CompositeField {
    pub ident: Ident,
    pub constraints: ConstraintGroup,
    pub instruction_constraints: ConstraintGroup,
    pub symbol: String,
    pub raw_field: syn::Field,
}

// A type of an account field.
#[derive(Debug, PartialEq)]
pub enum Ty {
    AccountInfo,
    ProgramState(ProgramStateTy),
    CpiState(CpiStateTy),
    ProgramAccount(ProgramAccountTy),
    Loader(LoaderTy),
    CpiAccount(CpiAccountTy),
    Sysvar(SysvarTy),
}

#[derive(Debug, PartialEq)]
pub enum SysvarTy {
    Clock,
    Rent,
    EpochSchedule,
    Fees,
    RecentBlockhashes,
    SlotHashes,
    SlotHistory,
    StakeHistory,
    Instructions,
    Rewards,
}

#[derive(Debug, PartialEq)]
pub struct ProgramStateTy {
    pub account_ident: Ident,
}

#[derive(Debug, PartialEq)]
pub struct CpiStateTy {
    pub account_ident: Ident,
}

#[derive(Debug, PartialEq)]
pub struct ProgramAccountTy {
    // The struct type of the account.
    pub account_ident: Ident,
}

#[derive(Debug, PartialEq)]
pub struct CpiAccountTy {
    // The struct type of the account.
    pub account_ident: Ident,
}

#[derive(Debug, PartialEq)]
pub struct LoaderTy {
    // The struct type of the account.
    pub account_ident: Ident,
}

#[derive(Debug)]
pub struct Error {
    pub name: String,
    pub raw_enum: ItemEnum,
    pub ident: Ident,
    pub codes: Vec<ErrorCode>,
    pub args: Option<ErrorArgs>,
}

#[derive(Debug)]
pub struct ErrorArgs {
    pub offset: LitInt,
}

impl Parse for ErrorArgs {
    fn parse(stream: ParseStream) -> ParseResult<Self> {
        let offset_span = stream.span();
        let offset = stream.call(Ident::parse_any)?;
        if offset.to_string().as_str() != "offset" {
            return Err(ParseError::new(offset_span, "expected keyword offset"));
        }
        stream.parse::<Token![=]>()?;
        Ok(ErrorArgs {
            offset: stream.parse()?,
        })
    }
}

#[derive(Debug)]
pub struct ErrorCode {
    pub id: u32,
    pub ident: Ident,
    pub msg: Option<String>,
}

// All well formed constraints on a single `Accounts` field.
#[derive(Debug, Default, Clone)]
pub struct ConstraintGroup {
    init: Option<ConstraintInit>,
    mutable: Option<ConstraintMut>,
    signer: Option<ConstraintSigner>,
    owner: Option<ConstraintOwner>,
    rent_exempt: Option<ConstraintRentExempt>,
    seeds: Option<ConstraintSeedsGroup>,
    executable: Option<ConstraintExecutable>,
    state: Option<ConstraintState>,
    associated: Option<ConstraintAssociatedGroup>,
    belongs_to: Vec<ConstraintBelongsTo>,
    literal: Vec<ConstraintLiteral>,
    raw: Vec<ConstraintRaw>,
    close: Option<ConstraintClose>,
}

impl ConstraintGroup {
    pub fn is_init(&self) -> bool {
        self.init.is_some()
    }

    pub fn is_mutable(&self) -> bool {
        self.mutable.is_some()
    }

    pub fn is_signer(&self) -> bool {
        self.signer.is_some()
    }

    pub fn is_close(&self) -> bool {
        self.close.is_some()
    }
}

// A single account constraint *after* merging all tokens into a well formed
// constraint. Some constraints like "associated" are defined by multiple
// tokens, so a merging phase is required.
#[derive(Debug)]
pub enum Constraint {
    Init(ConstraintInit),
    Mut(ConstraintMut),
    Signer(ConstraintSigner),
    BelongsTo(ConstraintBelongsTo),
    Literal(ConstraintLiteral),
    Raw(ConstraintRaw),
    Owner(ConstraintOwner),
    RentExempt(ConstraintRentExempt),
    Seeds(ConstraintSeedsGroup),
    Executable(ConstraintExecutable),
    State(ConstraintState),
    AssociatedGroup(ConstraintAssociatedGroup),
    Close(ConstraintClose),
}

// Constraint token is a single keyword in a `#[account(<TOKEN>)]` attribute.
#[derive(Debug)]
pub enum ConstraintToken {
    Init(Context<ConstraintInit>),
    Mut(Context<ConstraintMut>),
    Signer(Context<ConstraintSigner>),
    BelongsTo(Context<ConstraintBelongsTo>),
    Literal(Context<ConstraintLiteral>),
    Raw(Context<ConstraintRaw>),
    Owner(Context<ConstraintOwner>),
    RentExempt(Context<ConstraintRentExempt>),
    Seeds(Context<ConstraintSeeds>),
    Executable(Context<ConstraintExecutable>),
    State(Context<ConstraintState>),
    Close(Context<ConstraintClose>),
    Associated(Context<ConstraintAssociated>),
    AssociatedPayer(Context<ConstraintAssociatedPayer>),
    AssociatedSpace(Context<ConstraintAssociatedSpace>),
    AssociatedWith(Context<ConstraintAssociatedWith>),
}

impl Parse for ConstraintToken {
    fn parse(stream: ParseStream) -> ParseResult<Self> {
        accounts_parser::constraints::parse_token(stream)
    }
}

#[derive(Debug, Clone)]
pub struct ConstraintInit {}

#[derive(Debug, Clone)]
pub struct ConstraintMut {}

#[derive(Debug, Clone)]
pub struct ConstraintSigner {}

#[derive(Debug, Clone)]
pub struct ConstraintBelongsTo {
    pub join_target: Ident,
}

#[derive(Debug, Clone)]
pub struct ConstraintLiteral {
    pub lit: LitStr,
}

#[derive(Debug, Clone)]
pub struct ConstraintRaw {
    pub raw: Expr,
}

#[derive(Debug, Clone)]
pub struct ConstraintOwner {
    pub owner_target: Ident,
}

#[derive(Debug, Clone)]
pub enum ConstraintRentExempt {
    Enforce,
    Skip,
}

#[derive(Debug, Clone)]
pub struct ConstraintSeedsGroup {
    pub is_init: bool,
    pub seeds: Punctuated<Expr, Token![,]>,
    pub payer: Option<Ident>,
    pub space: Option<LitInt>,
}

#[derive(Debug, Clone)]
pub struct ConstraintSeeds {
    pub seeds: Punctuated<Expr, Token![,]>,
}

#[derive(Debug, Clone)]
pub struct ConstraintExecutable {}

#[derive(Debug, Clone)]
pub struct ConstraintState {
    pub program_target: Ident,
}

#[derive(Debug, Clone)]
pub struct ConstraintAssociatedGroup {
    pub is_init: bool,
    pub associated_target: Ident,
    pub associated_seeds: Vec<Ident>,
    pub payer: Option<Ident>,
    pub space: Option<LitInt>,
}

#[derive(Debug, Clone)]
pub struct ConstraintAssociated {
    pub target: Ident,
}

#[derive(Debug, Clone)]
pub struct ConstraintAssociatedPayer {
    pub target: Ident,
}

#[derive(Debug, Clone)]
pub struct ConstraintAssociatedWith {
    pub target: Ident,
}

#[derive(Debug, Clone)]
pub struct ConstraintAssociatedSpace {
    pub space: LitInt,
}

#[derive(Debug, Clone)]
pub struct ConstraintClose {
    pub sol_dest: Ident,
}

// Syntaxt context object for preserving metadata about the inner item.
#[derive(Debug, Clone)]
pub struct Context<T> {
    span: Span,
    inner: T,
}

impl<T> Context<T> {
    pub fn new(span: Span, inner: T) -> Self {
        Self { span, inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> Deref for Context<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> Spanned for Context<T> {
    fn span(&self) -> Span {
        self.span
    }
}
