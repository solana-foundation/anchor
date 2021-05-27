use crate::{
    Constraint, ConstraintAssociated, ConstraintAssociatedGroup, ConstraintAssociatedPayer,
    ConstraintAssociatedSpace, ConstraintAssociatedWith, ConstraintBelongsTo, ConstraintExecutable,
    ConstraintGroup, ConstraintInit, ConstraintLiteral, ConstraintMut, ConstraintOwner,
    ConstraintRaw, ConstraintRentExempt, ConstraintSeeds, ConstraintSigner, ConstraintState,
    Context,
};
use syn::ext::IdentExt;
use syn::parse::{Error as ParseError, Parse, ParseStream, Result as ParseResult};
use syn::spanned::Spanned;
use syn::{bracketed, Expr, Ident, LitStr, Token};

// Parses a single constraint from a parse stream for `#[account(<STREAM>)]`.
pub fn parse(stream: ParseStream) -> ParseResult<Constraint> {
    let is_lit = stream.peek(LitStr);
    if is_lit {
        let lit: LitStr = stream.parse()?;
        let c = Constraint::Literal(Context::new(lit.span(), ConstraintLiteral { lit }));
        return Ok(c);
    }

    let ident = stream.call(Ident::parse_any)?;
    let kw = ident.to_string();

    let c = match kw.as_str() {
        "init" => Constraint::Init(Context::new(ident.span(), ConstraintInit {})),
        "mut" => Constraint::Mut(Context::new(ident.span(), ConstraintMut {})),
        "signer" => Constraint::Signer(Context::new(ident.span(), ConstraintSigner {})),
        "executable" => Constraint::Executable(Context::new(ident.span(), ConstraintExecutable {})),
        _ => {
            stream.parse::<Token![=]>()?;
            let span = ident.span().join(stream.span()).unwrap_or(ident.span());
            match kw.as_str() {
                "belongs_to" | "has_one" => Constraint::BelongsTo(Context::new(
                    span,
                    ConstraintBelongsTo {
                        join_target: stream.parse()?,
                    },
                )),
                "owner" => Constraint::Owner(Context::new(
                    span,
                    ConstraintOwner {
                        owner_target: stream.parse()?,
                    },
                )),
                "rent_exempt" => Constraint::RentExempt(Context::new(
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
                "state" => Constraint::State(Context::new(
                    span,
                    ConstraintState {
                        program_target: stream.parse()?,
                    },
                )),
                "associated" => Constraint::Associated(Context::new(
                    span,
                    ConstraintAssociated {
                        target: stream.parse()?,
                    },
                )),
                "payer" => Constraint::AssociatedPayer(Context::new(
                    span,
                    ConstraintAssociatedPayer {
                        target: stream.parse()?,
                    },
                )),
                "with" => Constraint::AssociatedWith(Context::new(
                    span,
                    ConstraintAssociatedWith {
                        target: stream.parse()?,
                    },
                )),
                "space" => Constraint::AssociatedSpace(Context::new(
                    span,
                    ConstraintAssociatedSpace {
                        space: stream.parse()?,
                    },
                )),
                "seeds" => {
                    let seeds;
                    let bracket = bracketed!(seeds in stream);
                    Constraint::Seeds(Context::new(
                        span.join(bracket.span).unwrap_or(span),
                        ConstraintSeeds {
                            seeds: seeds.parse_terminated(Expr::parse)?,
                        },
                    ))
                }
                "constraint" => Constraint::Raw(Context::new(
                    span,
                    ConstraintRaw {
                        raw: stream.parse()?,
                    },
                )),
                _ => Err(ParseError::new(ident.span(), "Invalid attribute"))?,
            }
        }
    };

    Ok(c)
}

#[derive(Default)]
pub struct ConstraintGroupBuilder {
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
}

impl ConstraintGroupBuilder {
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

        let ConstraintGroupBuilder {
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
        } = self;

        let is_init = init.is_some();
        Ok(ConstraintGroup {
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
            associated: associated.map(|associated| ConstraintAssociatedGroup {
                is_init,
                associated_target: associated.target.clone(),
                associated_seeds: associated_with.iter().map(|s| s.target.clone()).collect(),
                payer: associated_payer.map(|p| p.target.clone()),
                space: associated_space.map(|s| s.space.clone()),
            }),
        })
    }

    pub fn add(&mut self, c: Constraint) -> ParseResult<()> {
        match c {
            Constraint::Init(c) => self.add_init(c),
            Constraint::Mut(c) => self.add_mut(c),
            Constraint::Signer(c) => self.add_signer(c),
            Constraint::BelongsTo(c) => self.add_belongs_to(c),
            Constraint::Literal(c) => self.add_literal(c),
            Constraint::Raw(c) => self.add_raw(c),
            Constraint::Owner(c) => self.add_owner(c),
            Constraint::RentExempt(c) => self.add_rent_exempt(c),
            Constraint::Seeds(c) => self.add_seeds(c),
            Constraint::Executable(c) => self.add_executable(c),
            Constraint::State(c) => self.add_state(c),
            Constraint::Associated(c) => self.add_associated(c),
            Constraint::AssociatedPayer(c) => self.add_associated_payer(c),
            Constraint::AssociatedSpace(c) => self.add_associated_space(c),
            Constraint::AssociatedWith(c) => self.add_associated_with(c),
            Constraint::AssociatedGroup(_) => panic!("Invariant violation"),
        }
    }

    fn add_init(&mut self, c: Context<ConstraintInit>) -> ParseResult<()> {
        if self.init.is_some() {
            return Err(ParseError::new(c.span(), "init already provided"));
        }
        self.init.replace(c);
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
        self.associated.replace(c);
        Ok(())
    }

    fn add_associated_payer(&mut self, c: Context<ConstraintAssociatedPayer>) -> ParseResult<()> {
        if self.associated.is_none() {
            return Err(ParseError::new(
                c.span(),
                "associated must be provided before payer",
            ));
        }
        if self.associated_payer.is_some() {
            return Err(ParseError::new(c.span(), "payer already provided"));
        }
        self.associated_payer.replace(c);
        Ok(())
    }

    fn add_associated_space(&mut self, c: Context<ConstraintAssociatedSpace>) -> ParseResult<()> {
        if self.associated.is_none() {
            return Err(ParseError::new(
                c.span(),
                "associated must be provided before space",
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
