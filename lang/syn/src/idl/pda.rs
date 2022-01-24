use crate::idl::*;
use crate::parser;
use crate::parser::context::CrateContext;
use crate::ConstraintSeedsGroup;
use crate::{AccountsStruct, Field};
use std::collections::HashMap;
use std::str::FromStr;
use syn::Expr;

// Parses a seeds constraint, extracting the IdlSeed types.
//
// Note: This implementation makes assumptions about the types that can be used
//       (e.g., no program-defined function calls in seeds).
//
//       This probably doesn't cover all cases. If you see a warning log, you
//       can add a new case here. In the worst case, we miss a seed and
//       the parser will treat the given seeds as empty and so clients will
//       simply fail to automatically populate the PDA accounts.
//
// Seed Assumptions: Seeds must be of one of the following forms:
//
// - instruction argument.
// - account context field pubkey.
// - account data, where the account is defined in the current program.
//   We make an exception for the SPL token program, since it is so common
//   and sometimes convenient to use fields as a seed (e.g. Auction house
//   program). In the case of nested structs/account data, all nested structs
//   must be defined in the current program as well.
// - byte string literal (e.g. b"MY_SEED").
// - byte string literal constant  (e.g. `pub const MY_SEED: [u8; 2] = *b"hi";`).
// - array constants.
//
pub fn parse(
    ctx: &CrateContext,
    accounts: &AccountsStruct,
    acc: &Field,
    seeds_feature: bool,
) -> Option<IdlPda> {
    if !seeds_feature {
        return None;
    }
    let pda_parser = PdaParser::new(ctx, accounts);
    acc.constraints
        .seeds
        .as_ref()
        .map(|s| pda_parser.parse(s))
        .unwrap_or(None)
}

struct PdaParser<'a> {
    ctx: &'a CrateContext,
    // Accounts context.
    accounts: &'a AccountsStruct,
    // Maps var name to var type. These are the instruction arguments in a
    // given accounts context.
    ix_args: HashMap<String, String>,
    // Constants available in the crate.
    const_names: Vec<String>,
    // All field names of the accounts in the accounts context.
    account_field_names: Vec<String>,
}

impl<'a> PdaParser<'a> {
    fn new(ctx: &'a CrateContext, accounts: &'a AccountsStruct) -> Self {
        // All the available sources of seeds.
        let ix_args = accounts.instruction_args().unwrap_or_default();
        let const_names: Vec<String> = ctx.consts().map(|c| c.ident.to_string()).collect();
        let account_field_names = accounts.field_names();

        Self {
            ctx,
            accounts,
            ix_args,
            const_names,
            account_field_names,
        }
    }

    fn parse(&self, seeds_grp: &ConstraintSeedsGroup) -> Option<IdlPda> {
        // Extract the idl seed types from the constraints.
        let seeds = seeds_grp
            .seeds
            .iter()
            .map(|s| self.parse_seed(s))
            .collect::<Option<Vec<_>>>()?;

        // Parse the program id from the constraints.
        let program_id = seeds_grp
            .program_seed
            .as_ref()
            .map(|pid| self.parse_seed(pid))
            .unwrap_or_default();

        // Done.
        Some(IdlPda { seeds, program_id })
    }

    fn parse_seed(&self, seed: &Expr) -> Option<IdlSeed> {
        match seed {
            Expr::MethodCall(_) => {
                let seed_path = parse_seed_path(seed)?;

                if self.is_instruction(&seed_path) {
                    self.parse_instruction(&seed_path)
                } else if self.is_const(&seed_path) {
                    self.parse_const(&seed_path)
                } else if self.is_account(&seed_path) {
                    self.parse_account(&seed_path)
                } else if self.is_str_literal(&seed_path) {
                    self.parse_str_literal(&seed_path)
                } else {
                    println!("WARNING: unexpected seed category for var: {:?}", seed_path);
                    None
                }
            }
            Expr::Reference(expr_reference) => self.parse_seed(&expr_reference.expr),
            Expr::Index(_) => {
                println!("WARNING: auto pda derivation not currently supported for slice literals");
                None
            }
            // Unknown type. Please file an issue.
            _ => {
                println!("WARNING: unexpected seed: {:?}", seed);
                None
            }
        }
    }

    fn parse_instruction(&self, seed_path: &SeedPath) -> Option<IdlSeed> {
        let idl_ty = IdlType::from_str(self.ix_args.get(&seed_path.name()).unwrap()).ok()?;
        Some(IdlSeed::Arg(IdlSeedArg {
            ty: idl_ty,
            path: seed_path.path(),
        }))
    }

    fn parse_const(&self, seed_path: &SeedPath) -> Option<IdlSeed> {
        // Pull in the constant value directly into the IDL.
        assert!(seed_path.components().is_empty());
        let const_item = self
            .ctx
            .consts()
            .find(|c| c.ident == seed_path.name())
            .unwrap();
        let idl_ty = IdlType::from_str(&parser::tts_to_string(&const_item.ty)).ok()?;
        let mut idl_ty_value = parser::tts_to_string(&const_item.expr);

        if let IdlType::Array(_ty, _size) = &idl_ty {
            // Convert str literal to array.
            if idl_ty_value.contains("b\"") {
                let components: Vec<&str> = idl_ty_value.split('b').collect();
                assert!(components.len() == 2);
                let mut str_lit = components[1].to_string();
                str_lit.retain(|c| c != '"');
                idl_ty_value = format!("{:?}", str_lit.as_bytes());
            }
        }

        Some(IdlSeed::Const(IdlSeedConst {
            ty: idl_ty,
            value: serde_json::from_str(&idl_ty_value).unwrap(),
        }))
    }

    fn parse_account(&self, seed_path: &SeedPath) -> Option<IdlSeed> {
        // Get the anchor account field from the derive accounts struct.
        let account_field = self
            .accounts
            .fields
            .iter()
            .find(|field| *field.ident() == seed_path.name())
            .unwrap();

        // Follow the path to find the seed type.
        let ty = {
            let mut path = seed_path.components();
            match path.len() {
                0 => IdlType::PublicKey,
                1 => {
                    // Name of the account struct.
                    let account = account_field.ty_name()?;
                    if account == "TokenAccount" {
                        assert!(path.len() == 1);
                        match path[0].as_str() {
                            "mint" => IdlType::PublicKey,
                            "amount" => IdlType::U64,
                            "authority" => IdlType::PublicKey,
                            "delegated_amount" => IdlType::U64,
                            _ => {
                                println!("WARNING: token field isn't supported: {}", &path[0]);
                                return None;
                            }
                        }
                    } else {
                        // Get the rust representation of the field's struct.
                        let strct = self.ctx.structs().find(|s| s.ident == account).unwrap();
                        parse_field_path(self.ctx, strct, &mut path)
                    }
                }
                _ => panic!("invariant violation"),
            }
        };

        Some(IdlSeed::Account(IdlSeedAccount {
            ty,
            account: account_field.ty_name(),
            path: seed_path.path(),
        }))
    }

    fn parse_str_literal(&self, seed_path: &SeedPath) -> Option<IdlSeed> {
        let mut var_name = seed_path.name();
        // Remove the byte `b` prefix if the string is of the form `b"seed".
        if var_name.starts_with("b\"") {
            var_name.remove(0);
        }
        let value_string: String = var_name.chars().filter(|c| *c != '"').collect();
        Some(IdlSeed::Const(IdlSeedConst {
            value: serde_json::Value::String(value_string),
            ty: IdlType::String,
        }))
    }

    fn is_instruction(&self, seed_path: &SeedPath) -> bool {
        self.ix_args.contains_key(&seed_path.name())
    }

    fn is_const(&self, seed_path: &SeedPath) -> bool {
        self.const_names.contains(&seed_path.name())
    }

    fn is_account(&self, seed_path: &SeedPath) -> bool {
        self.account_field_names.contains(&seed_path.name())
    }

    fn is_str_literal(&self, seed_path: &SeedPath) -> bool {
        seed_path.components().is_empty() && seed_path.name().contains('"')
    }
}

// SeedPath represents the deconstructed syntax of a single pda seed,
// consisting of a variable name and a vec of all the sub fields accessed
// on that variable name. For example, if a seed is `my_field.my_data.as_ref()`,
// then the field name is `my_field` and the vec of sub fields is `[my_data]`.
#[derive(Debug)]
struct SeedPath(String, Vec<String>);

impl SeedPath {
    fn name(&self) -> String {
        self.0.clone()
    }

    // Full path to the data this seed represents.
    fn path(&self) -> String {
        match self.1.len() {
            0 => self.0.clone(),
            _ => format!("{}.{}", self.name(), self.components().join(".")),
        }
    }

    // All path components for the subfields accessed on this seed.
    fn components(&self) -> &[String] {
        &self.1
    }
}

// Extracts the seed path from a single seed expression.
fn parse_seed_path(seed: &Expr) -> Option<SeedPath> {
    // Convert the seed into the raw string representation.
    let seed_str = parser::tts_to_string(&seed);

    // Break up the seed into each sub field component.
    let mut components: Vec<&str> = seed_str.split(" . ").collect();
    if components.len() <= 1 {
        println!("WARNING: seeds are in an unexpected format: {:?}", seed);
        return None;
    }

    // The name of the variable (or field).
    let name = components.remove(0).to_string();

    // The path to the seed (only if the `name` type is a struct).
    let mut path = Vec::new();
    while !components.is_empty() {
        let c = components.remove(0);
        if c.contains("()") {
            break;
        }
        path.push(c.to_string());
    }
    if path.len() == 1 && (path[0] == "key" || path[0] == "key()") {
        path = Vec::new();
    }

    Some(SeedPath(name, path))
}

fn parse_field_path(ctx: &CrateContext, strct: &syn::ItemStruct, path: &mut &[String]) -> IdlType {
    let field_name = &path[0];
    *path = &path[1..];

    // Get the type name for the field.
    let next_field = strct
        .fields
        .iter()
        .find(|f| &f.ident.clone().unwrap().to_string() == field_name)
        .unwrap();
    let next_field_ty_str = parser::tts_to_string(&next_field.ty);

    // The path is empty so this must be a primitive type.
    if path.is_empty() {
        return next_field_ty_str.parse().unwrap();
    }

    // Get the rust representation of hte field's struct.
    let strct = ctx
        .structs()
        .find(|s| s.ident == next_field_ty_str)
        .unwrap();

    parse_field_path(ctx, strct, path)
}
