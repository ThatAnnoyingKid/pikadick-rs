/// The representation of a platform
#[derive(Debug, Clone, Copy, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Platform {
    Pc,
    Xbox,
    Ps4,
}
