use nd_async_rusqlite::rusqlite::{
    Result as RusqliteResult,
    types::{
        FromSql,
        FromSqlError,
        FromSqlResult,
        ToSql,
        ToSqlOutput,
        ValueRef,
    },
};
use serenity::model::id::GuildId;
use std::num::NonZeroU64;

const EMPTY: &str = "empty";

/// A String wrapper for an optional [`GuildId`],
///
/// This is "[u64].to_string()" if a guild, or "empty" if not.
#[derive(Debug, Copy, Clone)]
pub struct OptionGuildIdString(pub Option<GuildId>);

impl From<Option<GuildId>> for OptionGuildIdString {
    fn from(guild_id: Option<GuildId>) -> Self {
        Self(guild_id)
    }
}

impl ToSql for OptionGuildIdString {
    fn to_sql(&self) -> RusqliteResult<ToSqlOutput<'_>> {
        match self.0 {
            Some(guild_id) => Ok(ToSqlOutput::from(guild_id.to_string())),
            None => Ok(ToSqlOutput::Borrowed(ValueRef::Text(EMPTY.as_bytes()))),
        }
    }
}

impl FromSql for OptionGuildIdString {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let text = value.as_str()?;
        if text == EMPTY {
            return Ok(Self(None));
        }
        match text.parse::<NonZeroU64>() {
            Ok(guild_id) => Ok(Self(Some(GuildId::from(guild_id)))),
            Err(e) => Err(FromSqlError::Other(Box::new(e))),
        }
    }
}
