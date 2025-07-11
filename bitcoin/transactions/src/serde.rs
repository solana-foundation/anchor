use serde::Deserialize;

pub fn serialize_u128<S>(num: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&num.to_string())
}

pub fn deserialize_u128<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = <std::string::String as Deserialize>::deserialize(deserializer)?;
    s.parse::<u128>().map_err(serde::de::Error::custom)
}

pub fn serialize_u64<S>(num: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&num.to_string())
}

pub fn deserialize_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = <std::string::String as Deserialize>::deserialize(deserializer)?;
    s.parse::<u64>().map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use crate::serde::{deserialize_u128, deserialize_u64, serialize_u128, serialize_u64};
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_serialize_deserialize_u128() {
        let value: u128 = 123456789012345678901234567890;
        let _ = serde_json::to_string(&value.to_string()).unwrap();

        #[derive(Serialize, Deserialize)]
        struct Wrapper(
            #[serde(
                serialize_with = "serialize_u128",
                deserialize_with = "deserialize_u128"
            )]
            u128,
        );

        let wrapper: Wrapper = serde_json::from_str(&format!("\"{}\"", value)).unwrap();
        assert_eq!(wrapper.0, value);
    }

    #[test]
    fn test_serialize_deserialize_u64() {
        let value: u64 = 9876543210;
        let _ = serde_json::to_string(&value.to_string()).unwrap();

        #[derive(Serialize, Deserialize)]
        struct Wrapper(
            #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")] u64,
        );

        let wrapper: Wrapper = serde_json::from_str(&format!("\"{}\"", value)).unwrap();
        assert_eq!(wrapper.0, value);
    }
}
