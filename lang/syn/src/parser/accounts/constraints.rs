use crate::{
    ConstraintAssociated, ConstraintAssociatedGroup, ConstraintAssociatedPayer,
    ConstraintAssociatedSpace, ConstraintAssociatedWith, ConstraintBelongsTo, ConstraintClose,
    ConstraintExecutable, ConstraintGroup, ConstraintInit, ConstraintLiteral, ConstraintMut,
    ConstraintOwner, ConstraintRaw, ConstraintRentExempt, ConstraintSeeds, ConstraintSeedsGroup,
    ConstraintSigner, ConstraintState, ConstraintToken, Context, Ty,
};
use syn::ext::IdentExt;
use syn::parse::{Error as ParseError, Parse, ParseStream, Result as ParseResult};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{bracketed, Expr, Ident, LitStr, Token};

pub fn parse(
    f: &syn::Field,
    f_ty: Option<&Ty>,
    has_instruction_api: bool,
) -> ParseResult<(ConstraintGroup, ConstraintGroup)> {
    let mut constraints = ConstraintGroupBuilder::new(f_ty);
    for attr in f.attrs.iter().filter(is_account) {
        for c in attr.parse_args_with(Punctuated::<ConstraintToken, Comma>::parse_terminated)? {
            constraints.add(c)?;
        }
    }
    let account_constraints = constraints.build()?;

    let mut constraints = ConstraintGroupBuilder::new(f_ty);
    for attr in f.attrs.iter().filter(is_instruction) {
        if !has_instruction_api {
            return Err(ParseError::new(
                attr.span(),
                "an instruction api must be declared",
            ));
        }
        for c in attr.parse_args_with(Punctuated::<ConstraintToken, Comma>::parse_terminated)? {
            constraints.add(c)?;
        }
    }
    let instruction_constraints = constraints.build()?;

    Ok((account_constraints, instruction_constraints))
}

pub fn is_account(attr: &&syn::Attribute) -> bool {
    attr.path
        .get_ident()
        .map_or(false, |ident| ident == "account")
}

pub fn is_instruction(attr: &&syn::Attribute) -> bool {
    attr.path
        .get_ident()
        .map_or(false, |ident| ident == "instruction")
}

// Parses a single constraint from a parse stream for `#[account(<STREAM>)]`.
pub fn parse_token(stream: ParseStream) -> ParseResult<ConstraintToken> {
    let is_lit = stream.peek(LitStr);
    if is_lit {
        let lit: LitStr = stream.parse()?;
        let c = ConstraintToken::Literal(Context::new(lit.span(), ConstraintLiteral { lit }));
        return Ok(c);
    }

    let ident = stream.call(Ident::parse_any)?;
    let kw = ident.to_string();

    let c = match kw.as_str() {
        "init" => ConstraintToken::Init(Context::new(ident.span(), ConstraintInit {})),
        "mut" => ConstraintToken::Mut(Context::new(ident.span(), ConstraintMut {})),
        "signer" => ConstraintToken::Signer(Context::new(ident.span(), ConstraintSigner {})),
        "executable" => {
            ConstraintToken::Executable(Context::new(ident.span(), ConstraintExecutable {}))
        }
        _ => {
            stream.parse::<Token![=]>()?;
            let span = ident.span().join(stream.span()).unwrap_or(ident.span());
            match kw.as_str() {
                "belongs_to" | "has_one" => ConstraintToken::BelongsTo(Context::new(
                    span,
                    ConstraintBelongsTo {
                        join_target: stream.parse()?,
                    },
                )),
                "owner" => ConstraintToken::Owner(Context::new(
                    span,
                    ConstraintOwner {
                        owner_target: stream.parse()?,
                    },
                )),
                "rent_exempt" => ConstraintToken::RentExempt(Context::new(
                    span,
                    match stream.parse::<Ident>()?.to_string().as_str() {
                        "skip" => ConstraintRentExempt::Skip,
                        "enforce" => ConstraintRentExempt::Enforce,
                        _ => {
                            return Err(ParseError::new(
                                span,
                                "rent_exempt must be either skip or enforce",
                            ))
                        }
                    },
                )),
                "state" => ConstraintToken::State(Context::new(
                    span,
                    ConstraintState {
                        program_target: stream.parse()?,
                    },
                )),
                "associated" => ConstraintToken::Associated(Context::new(
                    span,
                    ConstraintAssociated {
                        target: stream.parse()?,
                    },
                )),
                "payer" => ConstraintToken::AssociatedPayer(Context::new(
                    span,
                    ConstraintAssociatedPayer {
                        target: stream.parse()?,
                    },
                )),
                "with" => ConstraintToken::AssociatedWith(Context::new(
                    span,
                    ConstraintAssociatedWith {
                        target: stream.parse()?,
                    },
                )),
                "space" => ConstraintToken::AssociatedSpace(Context::new(
                    span,
                    ConstraintAssociatedSpace {
                        space: stream.parse()?,
                    },
                )),
                "seeds" => {
                    let seeds;
                    let bracket = bracketed!(seeds in stream);
                    ConstraintToken::Seeds(Context::new(
                        span.join(bracket.span).unwrap_or(span),
                        ConstraintSeeds {
                            seeds: seeds.parse_terminated(Expr::parse)?,
                        },
                    ))
                }
                "constraint" => ConstraintToken::Raw(Context::new(
                    span,
                    ConstraintRaw {
                        raw: stream.parse()?,
                    },
                )),
                "close" => ConstraintToken::Close(Context::new(
                    span,
                    ConstraintClose {
                        sol_dest: stream.parse()?,
                    },
                )),
                _ => Err(ParseError::new(ident.span(), "Invalid attribute"))?,
            }
        }
    };

    Ok(c)
}

#[derive(Default)]
pub struct ConstraintGroupBuilder<'ty> {
    pub f_ty: Option<&'ty Ty>,
    pub init: Option<Context<ConstraintInit>>,
    pub mutable: Option<Context<ConstraintMut>>,
    pub signer: Option<Context<ConstraintSigner>>,
    pub belongs_to: Vec<Context<ConstraintBelongsTo>>,
    pub literal: Vec<Context<ConstraintLiteral>>,
    pub raw: Vec<Context<ConstraintRaw>>,
    pub owner: Option<Context<ConstraintOwner>>,
    pub rent_exempt: Option<Context<ConstraintRentExempt>>,
    pub seeds: Option<Context<ConstraintSeeds>>,
    pub executable: Option<Context<ConstraintExecutable>>,
    pub state: Option<Context<ConstraintState>>,
    pub associated: Option<Context<ConstraintAssociated>>,
    pub associated_payer: Option<Context<ConstraintAssociatedPayer>>,
    pub associated_space: Option<Context<ConstraintAssociatedSpace>>,
    pub associated_with: Vec<Context<ConstraintAssociatedWith>>,
    pub close: Option<Context<ConstraintClose>>,
}

impl<'ty> ConstraintGroupBuilder<'ty> {
    pub fn new(f_ty: Option<&'ty Ty>) -> Self {
        Self {
            f_ty,
            init: None,
            mutable: None,
            signer: None,
            belongs_to: Vec::new(),
            literal: Vec::new(),
            raw: Vec::new(),
            owner: None,
            rent_exempt: None,
            seeds: None,
            executable: None,
            state: None,
            associated: None,
            associated_payer: None,
            associated_space: None,
            associated_with: Vec::new(),
            close: None,
        }
    }
    pub fn build(mut self) -> ParseResult<ConstraintGroup> {
        // Init implies mutable and rent exempt.
        if let Some(i) = &self.init {
            match self.mutable {
                Some(m) => {
                    return Err(ParseError::new(
                        m.span(),
                        "mut cannot be provided with init",
                    ))
                }
                None => self
                    .mutable
                    .replace(Context::new(i.span(), ConstraintMut {})),
            };
            if self.rent_exempt.is_none() {
                self.rent_exempt
                    .replace(Context::new(i.span(), ConstraintRentExempt::Enforce));
            }
        }
        if let Some(i) = &self.seeds {
            if self.init.is_some() && self.associated_payer.is_none() {
                return Err(ParseError::new(
                    i.span(),
                    "payer must be provided when creating a program derived address",
                ));
            }
        }

        let ConstraintGroupBuilder {
            f_ty: _,
            init,
            mutable,
            signer,
            belongs_to,
            literal,
            raw,
            owner,
            rent_exempt,
            seeds,
            executable,
            state,
            associated,
            associated_payer,
            associated_space,
            associated_with,
            close,
        } = self;

        // Converts Option<Context<T>> -> Option<T>.
        macro_rules! into_inner {
            ($opt:ident) => {
                $opt.map(|c| c.into_inner())
            };
            ($opt:expr) => {
                $opt.map(|c| c.into_inner())
            };
        }
        // Converts Vec<Context<T>> - Vec<T>.
        macro_rules! into_inner_vec {
            ($opt:ident) => {
                $opt.into_iter().map(|c| c.into_inner()).collect()
            };
        }

        let is_init = init.is_some();
        Ok(ConstraintGroup {
            init: into_inner!(init),
            mutable: into_inner!(mutable),
            signer: into_inner!(signer),
            belongs_to: into_inner_vec!(belongs_to),
            literal: into_inner_vec!(literal),
            raw: into_inner_vec!(raw),
            owner: into_inner!(owner),
            rent_exempt: into_inner!(rent_exempt),
            seeds: seeds.map(|c| ConstraintSeedsGroup {
                is_init,
                seeds: c.into_inner().seeds,
                payer: into_inner!(associated_payer.clone()).map(|a| a.target),
                space: associated_space.clone().map(|s| s.space.clone()),
            }),
            executable: into_inner!(executable),
            state: into_inner!(state),
            associated: associated.map(|associated| ConstraintAssociatedGroup {
                is_init,
                associated_target: associated.target.clone(),
                associated_seeds: associated_with.iter().map(|s| s.target.clone()).collect(),
                payer: associated_payer.map(|p| p.target.clone()),
                space: associated_space.map(|s| s.space.clone()),
            }),
            close: into_inner!(close),
        })
    }

    pub fn add(&mut self, c: ConstraintToken) -> ParseResult<()> {
        match c {
            ConstraintToken::Init(c) => self.add_init(c),
            ConstraintToken::Mut(c) => self.add_mut(c),
            ConstraintToken::Signer(c) => self.add_signer(c),
            ConstraintToken::BelongsTo(c) => self.add_belongs_to(c),
            ConstraintToken::Literal(c) => self.add_literal(c),
            ConstraintToken::Raw(c) => self.add_raw(c),
            ConstraintToken::Owner(c) => self.add_owner(c),
            ConstraintToken::RentExempt(c) => self.add_rent_exempt(c),
            ConstraintToken::Seeds(c) => self.add_seeds(c),
            ConstraintToken::Executable(c) => self.add_executable(c),
            ConstraintToken::State(c) => self.add_state(c),
            ConstraintToken::Associated(c) => self.add_associated(c),
            ConstraintToken::AssociatedPayer(c) => self.add_associated_payer(c),
            ConstraintToken::AssociatedSpace(c) => self.add_associated_space(c),
            ConstraintToken::AssociatedWith(c) => self.add_associated_with(c),
            ConstraintToken::Close(c) => self.add_close(c),
        }
    }

    fn add_init(&mut self, c: Context<ConstraintInit>) -> ParseResult<()> {
        if self.init.is_some() {
            return Err(ParseError::new(c.span(), "init already provided"));
        }
        self.init.replace(c);
        Ok(())
    }

    fn add_close(&mut self, c: Context<ConstraintClose>) -> ParseResult<()> {
        if !matches!(self.f_ty, Some(Ty::ProgramAccount(_)))
            && !matches!(self.f_ty, Some(Ty::Loader(_)))
        {
            return Err(ParseError::new(
                c.span(),
                "close must be on a ProgramAccount",
            ));
        }
        if self.mutable.is_none() {
            return Err(ParseError::new(
                c.span(),
                "mut must be provided before close",
            ));
        }
        if self.close.is_some() {
            return Err(ParseError::new(c.span(), "close already provided"));
        }
        self.close.replace(c);
        Ok(())
    }

    fn add_mut(&mut self, c: Context<ConstraintMut>) -> ParseResult<()> {
        if self.mutable.is_some() {
            return Err(ParseError::new(c.span(), "mut already provided"));
        }
        self.mutable.replace(c);
        Ok(())
    }

    fn add_signer(&mut self, c: Context<ConstraintSigner>) -> ParseResult<()> {
        if self.signer.is_some() {
            return Err(ParseError::new(c.span(), "signer already provided"));
        }
        self.signer.replace(c);
        Ok(())
    }

    fn add_belongs_to(&mut self, c: Context<ConstraintBelongsTo>) -> ParseResult<()> {
        if self
            .belongs_to
            .iter()
            .filter(|item| item.join_target == c.join_target)
            .count()
            > 0
        {
            return Err(ParseError::new(
                c.span(),
                "belongs_to target already provided",
            ));
        }
        self.belongs_to.push(c);
        Ok(())
    }

    fn add_literal(&mut self, c: Context<ConstraintLiteral>) -> ParseResult<()> {
        self.literal.push(c);
        Ok(())
    }

    fn add_raw(&mut self, c: Context<ConstraintRaw>) -> ParseResult<()> {
        self.raw.push(c);
        Ok(())
    }

    fn add_owner(&mut self, c: Context<ConstraintOwner>) -> ParseResult<()> {
        if self.owner.is_some() {
            return Err(ParseError::new(c.span(), "owner already provided"));
        }
        self.owner.replace(c);
        Ok(())
    }

    fn add_rent_exempt(&mut self, c: Context<ConstraintRentExempt>) -> ParseResult<()> {
        if self.rent_exempt.is_some() {
            return Err(ParseError::new(c.span(), "rent already provided"));
        }
        self.rent_exempt.replace(c);
        Ok(())
    }

    fn add_seeds(&mut self, c: Context<ConstraintSeeds>) -> ParseResult<()> {
        if self.seeds.is_some() {
            return Err(ParseError::new(c.span(), "seeds already provided"));
        }
        if self.associated.is_some() {
            return Err(ParseError::new(
                c.span(),
                "both seeds and associated cannot be defined together",
            ));
        }
        self.seeds.replace(c);
        Ok(())
    }

    fn add_executable(&mut self, c: Context<ConstraintExecutable>) -> ParseResult<()> {
        if self.executable.is_some() {
            return Err(ParseError::new(c.span(), "executable already provided"));
        }
        self.executable.replace(c);
        Ok(())
    }

    fn add_state(&mut self, c: Context<ConstraintState>) -> ParseResult<()> {
        if self.state.is_some() {
            return Err(ParseError::new(c.span(), "state already provided"));
        }
        self.state.replace(c);
        Ok(())
    }

    fn add_associated(&mut self, c: Context<ConstraintAssociated>) -> ParseResult<()> {
        if self.associated.is_some() {
            return Err(ParseError::new(c.span(), "associated already provided"));
        }
        if self.seeds.is_some() {
            return Err(ParseError::new(
                c.span(),
                "both seeds and associated cannot be defined together",
            ));
        }
        self.associated.replace(c);
        Ok(())
    }

    fn add_associated_payer(&mut self, c: Context<ConstraintAssociatedPayer>) -> ParseResult<()> {
        if self.associated.is_none() && self.seeds.is_none() {
            return Err(ParseError::new(
                c.span(),
                "associated or seeds must be provided before payer",
            ));
        }
        if self.associated_payer.is_some() {
            return Err(ParseError::new(c.span(), "payer already provided"));
        }
        self.associated_payer.replace(c);
        Ok(())
    }

    fn add_associated_space(&mut self, c: Context<ConstraintAssociatedSpace>) -> ParseResult<()> {
        if self.associated.is_none() && self.seeds.is_none() {
            return Err(ParseError::new(
                c.span(),
                "associated or seeds must be provided before space",
            ));
        }
        if self.associated_space.is_some() {
            return Err(ParseError::new(c.span(), "space already provided"));
        }
        self.associated_space.replace(c);
        Ok(())
    }

    fn add_associated_with(&mut self, c: Context<ConstraintAssociatedWith>) -> ParseResult<()> {
        if self.associated.is_none() {
            return Err(ParseError::new(
                c.span(),
                "associated must be provided before with",
            ));
        }
        self.associated_with.push(c);
        Ok(())
    }
}
