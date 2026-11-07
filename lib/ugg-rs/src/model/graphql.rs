use serde::{
    Deserializer,
    de::Error,
};

#[derive(Debug, serde::Deserialize)]
pub struct Location {
    pub column: u32,
    pub line: u32,
}

#[derive(Debug, serde::Deserialize)]
pub struct GraphQlError {
    pub locations: Vec<Location>,
    pub message: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GraphQlResponseError {
    pub errors: Vec<GraphQlError>,
}

impl std::fmt::Display for GraphQlResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(
            self.errors
                .first()
                .map(|error| error.message.as_str())
                .unwrap_or("GraphQL error response missing error"),
        )
    }
}

impl std::error::Error for GraphQlResponseError {}

#[derive(Debug)]
pub enum GraphQlResponse<T> {
    Ok(T),
    Error(GraphQlResponseError),
}

impl<T> GraphQlResponse<T> {
    pub fn into_result(self) -> Result<T, GraphQlResponseError> {
        match self {
            Self::Ok(data) => Ok(data),
            Self::Error(error) => Err(error),
        }
    }
}

impl<'de, T> serde::de::Deserialize<'de> for GraphQlResponse<T>
where
    T: serde::de::DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<GraphQlResponse<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, serde::Deserialize)]
        struct GraphQlResponseOk<T> {
            data: T,
        }

        let value = serde_json::Value::deserialize(deserializer)?;
        let is_valid = value
            .as_object()
            .is_some_and(|obj| obj.contains_key("data"));

        if is_valid {
            let response_ok: GraphQlResponseOk<T> =
                serde_json::from_value(value).map_err(D::Error::custom)?;
            Ok(Self::Ok(response_ok.data))
        } else {
            let response_error: GraphQlResponseError =
                serde_json::from_value(value).map_err(D::Error::custom)?;
            Ok(Self::Error(response_error))
        }
    }
}
