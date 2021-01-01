use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Idl {
    pub version: String,
    pub name: String,
    pub instructions: Vec<IdlInstruction>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub accounts: Vec<IdlTypeDef>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub types: Vec<IdlTypeDef>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdlInstruction {
    pub name: String,
    pub accounts: Vec<IdlAccount>,
    pub args: Vec<IdlField>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
pub struct IdlTypeDef {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlTypeDefTy,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "kind")]
pub enum IdlTypeDefTy {
    Struct { fields: Vec<IdlField> },
    Enum { variants: Vec<EnumVariant> },
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
    Option(Box<IdlType>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdlTypePublicKey;

impl std::str::FromStr for IdlType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Eliminate whitespace.
        let mut s = s.to_string();
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
            "Vec<u8>" => IdlType::Bytes,
            "String" => IdlType::String,
            "Pubkey" => IdlType::PublicKey,
            _ => match s.to_string().strip_prefix("Option<") {
                None => IdlType::Defined(s.to_string()),
                Some(inner) => {
                    let inner_ty = Self::from_str(
                        inner
                            .strip_suffix(">")
                            .ok_or(anyhow::anyhow!("Invalid option"))?,
                    )?;
                    IdlType::Option(Box::new(inner_ty))
                }
            },
        };
        Ok(r)
    }
}
