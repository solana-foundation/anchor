use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub mod file;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Idl {
    pub version: String,
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub constants: Vec<IdlConst>,
    pub instructions: Vec<IdlInstruction>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<IdlState>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub accounts: Vec<IdlTypeDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub types: Vec<IdlTypeDefinition>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub events: Option<Vec<IdlEvent>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub errors: Option<Vec<IdlErrorCode>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdlConst {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlType,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdlState {
    #[serde(rename = "struct")]
    pub strct: IdlTypeDefinition,
    pub methods: Vec<IdlInstruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdlInstruction {
    pub name: String,
    pub accounts: Vec<IdlAccountItem>,
    pub args: Vec<IdlField>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdlField {
    pub name: String,
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
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlTypeDefinitionTy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase", tag = "kind")]
pub enum IdlTypeDefinitionTy {
    Struct { fields: Vec<IdlField> },
    Enum { variants: Vec<IdlEnumVariant> },
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
    U64,
    I64,
    U128,
    I128,
    Bytes,
    String,
    PublicKey,
    Defined(String),
    Option(Box<IdlType>),
    Vec(Box<IdlType>),
    Array(Box<IdlType>, usize),
}

impl std::str::FromStr for IdlType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.to_string();
        fn array_from_str(inner: &str) -> IdlType {
            match inner.strip_suffix(']') {
                None => {
                    let (raw_type, raw_length) = inner.rsplit_once(';').unwrap();
                    let ty = IdlType::from_str(raw_type).unwrap();
                    let len = raw_length.replace("_", "").parse::<usize>().unwrap();
                    IdlType::Array(Box::new(ty), len)
                }
                Some(nested_inner) => array_from_str(&nested_inner[1..]),
            }
        }
        s.retain(|c| !c.is_whitespace());
        let r = match s.as_str() {
            "bool" => IdlType::Bool,
            "u8" => IdlType::U8,
            "i8" => IdlType::I8,
            "u16" => IdlType::U16,
            "i16" => IdlType::I16,
            "u32" => IdlType::U32,
            "i32" => IdlType::I32,
            "u64" => IdlType::U64,
            "i64" => IdlType::I64,
            "u128" => IdlType::U128,
            "i128" => IdlType::I128,
            "Vec<u8>" => IdlType::Bytes,
            "String" => IdlType::String,
            "Pubkey" => IdlType::PublicKey,
            _ => match s.to_string().strip_prefix("Option<") {
                None => match s.to_string().strip_prefix("Vec<") {
                    None => {
                        if s.to_string().starts_with('[') {
                            array_from_str(&s)
                        } else {
                            IdlType::Defined(s.to_string())
                        }
                    }
                    Some(inner) => {
                        let inner_ty = Self::from_str(
                            inner
                                .strip_suffix('>')
                                .ok_or_else(|| anyhow::anyhow!("Invalid option"))?,
                        )?;
                        IdlType::Vec(Box::new(inner_ty))
                    }
                },
                Some(inner) => {
                    let inner_ty = Self::from_str(
                        inner
                            .strip_suffix('>')
                            .ok_or_else(|| anyhow::anyhow!("Invalid option"))?,
                    )?;
                    IdlType::Option(Box::new(inner_ty))
                }
            },
        };
        Ok(r)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdlErrorCode {
    pub code: u32,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub msg: Option<String>,
}

#[cfg(test)]
mod tests {
    use crate::idl::IdlType;
    use std::str::FromStr;

    #[test]
    fn multidimensional_array() {
        assert_eq!(
            IdlType::from_str("[[u8;16];32]").unwrap(),
            IdlType::Array(Box::new(IdlType::Array(Box::new(IdlType::U8), 16)), 32)
        );
    }

    #[test]
    fn array() {
        assert_eq!(
            IdlType::from_str("[Pubkey;16]").unwrap(),
            IdlType::Array(Box::new(IdlType::PublicKey), 16)
        );
    }

    #[test]
    fn array_with_underscored_length() {
        assert_eq!(
            IdlType::from_str("[u8;50_000]").unwrap(),
            IdlType::Array(Box::new(IdlType::U8), 50000)
        );
    }

    #[test]
    fn option() {
        assert_eq!(
            IdlType::from_str("Option<bool>").unwrap(),
            IdlType::Option(Box::new(IdlType::Bool))
        )
    }

    #[test]
    fn vector() {
        assert_eq!(
            IdlType::from_str("Vec<bool>").unwrap(),
            IdlType::Vec(Box::new(IdlType::Bool))
        )
    }
}
