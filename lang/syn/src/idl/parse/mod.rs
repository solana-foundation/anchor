use crate::idl::types::*;

pub mod file;
pub mod pda;
pub mod relations;

impl std::str::FromStr for IdlType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.to_string();
        fn array_from_str(inner: &str) -> IdlType {
            match inner.strip_suffix(']') {
                None => {
                    let (raw_type, raw_length) = inner.rsplit_once(';').unwrap();
                    let ty = IdlType::from_str(raw_type).unwrap();
                    let len = raw_length.replace('_', "").parse::<usize>().unwrap();
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
            "f32" => IdlType::F32,
            "u64" => IdlType::U64,
            "i64" => IdlType::I64,
            "f64" => IdlType::F64,
            "u128" => IdlType::U128,
            "i128" => IdlType::I128,
            "u256" => IdlType::U256,
            "i256" => IdlType::I256,
            "Vec<u8>" => IdlType::Bytes,
            "String" | "&str" | "&'staticstr" => IdlType::String,
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

#[cfg(test)]
mod tests {
    use crate::idl::types::IdlType;
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
