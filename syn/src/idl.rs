use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Idl {
    pub version: String,
    pub name: String,
    pub methods: Vec<IdlMethod>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub accounts: Vec<IdlTypeDef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub types: Vec<IdlTypeDef>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdlMethod {
    pub name: String,
    pub accounts: Vec<IdlAccount>,
    pub args: Vec<IdlField>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdlAccount {
    pub name: String,
    pub is_mut: bool,
    pub is_signer: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdlField {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum IdlTypeDef {
    Struct {
        name: String,
        fields: Vec<IdlField>,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fields: Option<EnumFields>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnumFields {
    Named(Vec<IdlField>),
    Tuple(Vec<IdlType>),
}

#[derive(Debug, Serialize, Deserialize)]
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
    Bytes,
    String,
    PublicKey,
    Defined(String),
}

impl std::str::FromStr for IdlType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let r = match s {
            "bool" => IdlType::Bool,
            "u8" => IdlType::U8,
            "i8" => IdlType::I8,
            "u16" => IdlType::U16,
            "i16" => IdlType::I16,
            "u32" => IdlType::U32,
            "I32" => IdlType::I32,
            "u64" => IdlType::U64,
            "i64" => IdlType::I64,
            "Vec<u8>" => IdlType::Bytes,
            "String" => IdlType::String,
            "Pubkey" => IdlType::PublicKey,
            _ => IdlType::Defined(s.to_string()),
        };
        Ok(r)
    }
}
