pub(crate) mod from_str {
    use serde::{
        de::Error,
        Deserialize,
        Deserializer,
    };
    use std::str::FromStr;

    pub(crate) fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr,
        <T as FromStr>::Err: std::fmt::Display,
    {
        let s = String::deserialize(deserializer)?;
        T::from_str(&s).map_err(Error::custom)
    }

    pub fn serialize<T, S>(value: T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        T: ToString,
    {
        let value = value.to_string();
        serializer.serialize_str(value.as_str())
    }
}
