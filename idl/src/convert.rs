use anyhow::{anyhow, Result};

use crate::types::Idl;

/// Create an [`Idl`] value with additional support for older specs based on the
/// `idl.metadata.spec` field.
///
/// If `spec` field is not specified, the conversion will fallback to the legacy IDL spec
/// (pre Anchor v0.30).
///
/// **Note:** For legacy IDLs, `idl.metadata.address` field is required to be populated with
/// program's address otherwise an error will be returned.
pub fn convert_idl(idl: &[u8]) -> Result<Idl> {
    let value = serde_json::from_slice::<serde_json::Value>(idl)?;
    let spec = value
        .get("metadata")
        .and_then(|m| m.get("spec"))
        .and_then(|spec| spec.as_str());
    match spec {
        // New standard
        Some(spec) => match spec {
            "0.1.0" => serde_json::from_value(value).map_err(Into::into),
            _ => Err(anyhow!("IDL spec not supported: `{spec}`")),
        },
        // Legacy
        None => serde_json::from_value::<legacy::Idl>(value).map(TryInto::try_into)?,
    }
}

/// Legacy IDL spec (pre Anchor v0.30)
mod legacy {
    use crate::types as t;
    use anyhow::{anyhow, Result};
    use heck::SnakeCase;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct Idl {
        pub version: String,
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub docs: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        pub constants: Vec<IdlConst>,
        pub instructions: Vec<IdlInstruction>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        pub accounts: Vec<IdlTypeDefinition>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        pub types: Vec<IdlTypeDefinition>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub events: Option<Vec<IdlEvent>>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub errors: Option<Vec<IdlErrorCode>>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub metadata: Option<serde_json::Value>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct IdlConst {
        pub name: String,
        #[serde(rename = "type")]
        pub ty: IdlType,
        pub value: String,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct IdlState {
        #[serde(rename = "struct")]
        pub strct: IdlTypeDefinition,
        pub methods: Vec<IdlInstruction>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct IdlInstruction {
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub docs: Option<Vec<String>>,
        pub accounts: Vec<IdlAccountItem>,
        pub args: Vec<IdlField>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub returns: Option<IdlType>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct IdlAccounts {
        pub name: String,
        pub accounts: Vec<IdlAccountItem>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(untagged)]
    pub enum IdlAccountItem {
        IdlAccount(IdlAccount),
        IdlAccounts(IdlAccounts),
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct IdlAccount {
        pub name: String,
        pub is_mut: bool,
        pub is_signer: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub is_optional: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub docs: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub pda: Option<IdlPda>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        pub relations: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct IdlPda {
        pub seeds: Vec<IdlSeed>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub program_id: Option<IdlSeed>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase", tag = "kind")]
    pub enum IdlSeed {
        Const(IdlSeedConst),
        Arg(IdlSeedArg),
        Account(IdlSeedAccount),
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct IdlSeedAccount {
        #[serde(rename = "type")]
        pub ty: IdlType,
        // account_ty points to the entry in the "accounts" section.
        // Some only if the `Account<T>` type is used.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub account: Option<String>,
        pub path: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct IdlSeedArg {
        #[serde(rename = "type")]
        pub ty: IdlType,
        pub path: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct IdlSeedConst {
        #[serde(rename = "type")]
        pub ty: IdlType,
        pub value: serde_json::Value,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct IdlField {
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub docs: Option<Vec<String>>,
        #[serde(rename = "type")]
        pub ty: IdlType,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct IdlEvent {
        pub name: String,
        pub fields: Vec<IdlEventField>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct IdlEventField {
        pub name: String,
        #[serde(rename = "type")]
        pub ty: IdlType,
        pub index: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct IdlTypeDefinition {
        /// - `idl-parse`: always the name of the type
        /// - `idl-build`: full path if there is a name conflict, otherwise the name of the type
        pub name: String,
        /// Documentation comments
        #[serde(skip_serializing_if = "Option::is_none")]
        pub docs: Option<Vec<String>>,
        /// Generics, only supported with `idl-build`
        #[serde(skip_serializing_if = "Option::is_none")]
        pub generics: Option<Vec<String>>,
        /// Type definition, `struct` or `enum`
        #[serde(rename = "type")]
        pub ty: IdlTypeDefinitionTy,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "lowercase", tag = "kind")]
    pub enum IdlTypeDefinitionTy {
        Struct { fields: Vec<IdlField> },
        Enum { variants: Vec<IdlEnumVariant> },
        Alias { value: IdlType },
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct IdlEnumVariant {
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub fields: Option<EnumFields>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(untagged)]
    pub enum EnumFields {
        Named(Vec<IdlField>),
        Tuple(Vec<IdlType>),
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub enum IdlType {
        Bool,
        U8,
        I8,
        U16,
        I16,
        U32,
        I32,
        F32,
        U64,
        I64,
        F64,
        U128,
        I128,
        U256,
        I256,
        Bytes,
        String,
        PublicKey,
        Defined(String),
        Option(Box<IdlType>),
        Vec(Box<IdlType>),
        Array(Box<IdlType>, usize),
        GenericLenArray(Box<IdlType>, String),
        Generic(String),
        DefinedWithTypeArgs {
            name: String,
            args: Vec<IdlDefinedTypeArg>,
        },
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub enum IdlDefinedTypeArg {
        Generic(String),
        Value(String),
        Type(IdlType),
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct IdlErrorCode {
        pub code: u32,
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub msg: Option<String>,
    }

    impl TryFrom<Idl> for t::Idl {
        type Error = anyhow::Error;

        fn try_from(idl: Idl) -> Result<Self> {
            Ok(Self {
                address: idl
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("address"))
                    .and_then(|a| a.as_str())
                    .ok_or_else(|| anyhow!("Program id missing in `idl.metadata.address` field"))?
                    .into(),
                metadata: t::IdlMetadata {
                    name: idl.name,
                    version: idl.version,
                    spec: t::IDL_SPEC.into(),
                    description: Default::default(),
                    repository: Default::default(),
                    dependencies: Default::default(),
                    contact: Default::default(),
                    deployments: Default::default(),
                },
                docs: idl.docs.unwrap_or_default(),
                instructions: idl.instructions.into_iter().map(Into::into).collect(),
                accounts: idl.accounts.clone().into_iter().map(Into::into).collect(),
                events: idl
                    .events
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                errors: idl
                    .errors
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                types: idl
                    .types
                    .into_iter()
                    .map(Into::into)
                    .chain(idl.accounts.into_iter().map(Into::into))
                    .chain(idl.events.unwrap_or_default().into_iter().map(Into::into))
                    .collect(),
                constants: idl.constants.into_iter().map(Into::into).collect(),
            })
        }
    }

    fn get_disc(prefix: &str, name: &str) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(prefix);
        hasher.update(b":");
        hasher.update(name);
        hasher.finalize()[..8].into()
    }

    impl From<IdlInstruction> for t::IdlInstruction {
        fn from(value: IdlInstruction) -> Self {
            let snake_name = value.name.to_snake_case();
            let harmonized_name = harmonized_camel_case(&value.name);
            Self {
                discriminator: get_disc("global", &snake_name),
                name: harmonized_name,
                docs: value.docs.unwrap_or_default(),
                accounts: value.accounts.into_iter().map(Into::into).collect(),
                args: value.args.into_iter().map(Into::into).collect(),
                returns: value.returns.map(|r| r.into()),
            }
        }
    }

    /// Converts snake_case, SCREAMING_SNAKE_CASE, or PascalCase to camelCase.
    ///
    /// - First letter is lowercased
    /// - Letters following underscores are capitalized (snake_case handling)
    /// - Letters following digits are capitalized (e.g., a1b_receive → a1BReceive)
    /// - For inputs with underscores (snake_case/SCREAMING_SNAKE_CASE), letters are lowercased
    ///   except at word boundaries (MY_CONST → myConst)
    /// - For inputs without underscores (PascalCase), internal capitalization is preserved
    ///   (DummyA → dummyA)
    ///
    /// This ensures consistent naming between Rust IDL generation and TypeScript clients.
    fn harmonized_camel_case(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut capitalize_next = false;
        let mut prev_was_digit = false;
        let mut is_first = true;
        // If input has underscores, treat as snake_case/SCREAMING_SNAKE_CASE and lowercase letters
        let has_underscore = input.contains('_');

        for c in input.chars() {
            if c == '_' {
                capitalize_next = true;
                prev_was_digit = false;
            } else if c.is_ascii_digit() {
                result.push(c);
                prev_was_digit = true;
                is_first = false;
            } else if c.is_ascii_alphabetic() {
                if is_first {
                    // First letter is always lowercase for camelCase
                    result.push(c.to_ascii_lowercase());
                } else if capitalize_next || prev_was_digit {
                    // After underscore or digit, always capitalize
                    result.push(c.to_ascii_uppercase());
                } else if has_underscore {
                    // For snake_case/SCREAMING_SNAKE_CASE, lowercase within words
                    result.push(c.to_ascii_lowercase());
                } else {
                    // For PascalCase (no underscores), preserve original case
                    result.push(c);
                }
                capitalize_next = false;
                prev_was_digit = false;
                is_first = false;
            } else {
                result.push(c);
                capitalize_next = false;
                prev_was_digit = false;
                is_first = false;
            }
        }
        result
    }

    impl From<IdlTypeDefinition> for t::IdlAccount {
        fn from(value: IdlTypeDefinition) -> Self {
            Self {
                discriminator: get_disc("account", &value.name),
                name: value.name,
            }
        }
    }

    impl From<IdlEvent> for t::IdlEvent {
        fn from(value: IdlEvent) -> Self {
            Self {
                discriminator: get_disc("event", &value.name),
                name: value.name,
            }
        }
    }

    impl From<IdlErrorCode> for t::IdlErrorCode {
        fn from(value: IdlErrorCode) -> Self {
            Self {
                name: value.name,
                code: value.code,
                msg: value.msg,
            }
        }
    }

    impl From<IdlConst> for t::IdlConst {
        fn from(value: IdlConst) -> Self {
            Self {
                name: value.name,
                docs: Default::default(),
                ty: value.ty.into(),
                value: value.value,
            }
        }
    }

    impl From<IdlDefinedTypeArg> for t::IdlGenericArg {
        fn from(value: IdlDefinedTypeArg) -> Self {
            match value {
                IdlDefinedTypeArg::Type(ty) => Self::Type { ty: ty.into() },
                IdlDefinedTypeArg::Value(value) => Self::Const { value },
                IdlDefinedTypeArg::Generic(generic) => Self::Type {
                    ty: t::IdlType::Generic(generic),
                },
            }
        }
    }

    impl From<IdlTypeDefinition> for t::IdlTypeDef {
        fn from(value: IdlTypeDefinition) -> Self {
            Self {
                name: value.name,
                docs: value.docs.unwrap_or_default(),
                serialization: Default::default(),
                repr: Default::default(),
                generics: Default::default(),
                ty: value.ty.into(),
            }
        }
    }

    impl From<IdlEvent> for t::IdlTypeDef {
        fn from(value: IdlEvent) -> Self {
            Self {
                name: value.name,
                docs: Default::default(),
                serialization: Default::default(),
                repr: Default::default(),
                generics: Default::default(),
                ty: t::IdlTypeDefTy::Struct {
                    fields: Some(t::IdlDefinedFields::Named(
                        value
                            .fields
                            .into_iter()
                            .map(|f| t::IdlField {
                                name: f.name.to_snake_case(),
                                docs: Default::default(),
                                ty: f.ty.into(),
                            })
                            .collect(),
                    )),
                },
            }
        }
    }

    impl From<IdlTypeDefinitionTy> for t::IdlTypeDefTy {
        fn from(value: IdlTypeDefinitionTy) -> Self {
            match value {
                IdlTypeDefinitionTy::Struct { fields } => Self::Struct {
                    fields: if fields.is_empty() {
                        None
                    } else {
                        Some(t::IdlDefinedFields::Named(
                            fields.into_iter().map(Into::into).collect(),
                        ))
                    },
                },
                IdlTypeDefinitionTy::Enum { variants } => Self::Enum {
                    variants: variants
                        .into_iter()
                        .map(|variant| t::IdlEnumVariant {
                            name: variant.name,
                            fields: variant.fields.map(|fields| match fields {
                                EnumFields::Named(fields) => t::IdlDefinedFields::Named(
                                    fields.into_iter().map(Into::into).collect(),
                                ),
                                EnumFields::Tuple(tys) => t::IdlDefinedFields::Tuple(
                                    tys.into_iter().map(Into::into).collect(),
                                ),
                            }),
                        })
                        .collect(),
                },
                IdlTypeDefinitionTy::Alias { value } => Self::Type {
                    alias: value.into(),
                },
            }
        }
    }

    impl From<IdlField> for t::IdlField {
        fn from(value: IdlField) -> Self {
            Self {
                name: value.name.to_snake_case(),
                docs: value.docs.unwrap_or_default(),
                ty: value.ty.into(),
            }
        }
    }

    impl From<IdlType> for t::IdlType {
        fn from(value: IdlType) -> Self {
            match value {
                IdlType::PublicKey => t::IdlType::Pubkey,
                IdlType::Defined(name) => t::IdlType::Defined {
                    name,
                    generics: Default::default(),
                },
                IdlType::DefinedWithTypeArgs { name, args } => t::IdlType::Defined {
                    name,
                    generics: args.into_iter().map(Into::into).collect(),
                },
                IdlType::Option(ty) => t::IdlType::Option(ty.into()),
                IdlType::Vec(ty) => t::IdlType::Vec(ty.into()),
                IdlType::Array(ty, len) => t::IdlType::Array(ty.into(), t::IdlArrayLen::Value(len)),
                IdlType::GenericLenArray(ty, generic) => {
                    t::IdlType::Array(ty.into(), t::IdlArrayLen::Generic(generic))
                }
                _ => serde_json::to_value(value)
                    .and_then(serde_json::from_value)
                    .unwrap(),
            }
        }
    }

    impl From<Box<IdlType>> for Box<t::IdlType> {
        fn from(value: Box<IdlType>) -> Self {
            Box::new((*value).into())
        }
    }

    impl From<IdlAccountItem> for t::IdlInstructionAccountItem {
        fn from(value: IdlAccountItem) -> Self {
            match value {
                IdlAccountItem::IdlAccount(acc) => Self::Single(t::IdlInstructionAccount {
                    name: acc.name.to_snake_case(),
                    docs: acc.docs.unwrap_or_default(),
                    writable: acc.is_mut,
                    signer: acc.is_signer,
                    optional: acc.is_optional.unwrap_or_default(),
                    address: Default::default(),
                    pda: acc
                        .pda
                        .map(|pda| -> Result<t::IdlPda> {
                            Ok(t::IdlPda {
                                seeds: pda
                                    .seeds
                                    .into_iter()
                                    .map(TryInto::try_into)
                                    .collect::<Result<_>>()?,
                                program: pda.program_id.map(TryInto::try_into).transpose()?,
                            })
                        })
                        .transpose()
                        .unwrap_or_default(),
                    relations: acc.relations,
                }),
                IdlAccountItem::IdlAccounts(accs) => Self::Composite(t::IdlInstructionAccounts {
                    name: accs.name.to_snake_case(),
                    accounts: accs.accounts.into_iter().map(Into::into).collect(),
                }),
            }
        }
    }

    impl TryFrom<IdlSeed> for t::IdlSeed {
        type Error = anyhow::Error;

        fn try_from(value: IdlSeed) -> Result<Self> {
            let seed = match value {
                IdlSeed::Account(seed) => Self::Account(t::IdlSeedAccount {
                    account: seed.account,
                    path: seed.path,
                }),
                IdlSeed::Arg(seed) => Self::Arg(t::IdlSeedArg { path: seed.path }),
                IdlSeed::Const(seed) => Self::Const(t::IdlSeedConst {
                    value: match seed.ty {
                        IdlType::String => seed.value.to_string().as_bytes().into(),
                        _ => return Err(anyhow!("Const seed conversion not supported")),
                    },
                }),
            };
            Ok(seed)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_harmonized_camel_case() {
            assert_eq!(harmonized_camel_case("a1b_receive"), "a1BReceive");
            assert_eq!(harmonized_camel_case("test2var"), "test2Var");

            // PascalCase inputs (preserve internal capitalization)
            assert_eq!(harmonized_camel_case("DummyA"), "dummyA");
            assert_eq!(harmonized_camel_case("Initialize"), "initialize");

            // SCREAMING_SNAKE_CASE inputs (constants)
            assert_eq!(harmonized_camel_case("MY_CONST"), "myConst");
            assert_eq!(harmonized_camel_case("BYTE_STR"), "byteStr");
            assert_eq!(harmonized_camel_case("U8"), "u8");
        }
    }
}
