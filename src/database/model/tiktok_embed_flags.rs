use bitflags::bitflags;
use nd_async_rusqlite::rusqlite::{
    ToSql,
    types::{
        FromSql,
        FromSqlError,
        FromSqlResult,
        ToSqlOutput,
        ValueRef,
    },
};

bitflags! {
    /// Flags for TikTok embeds
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    pub struct TikTokEmbedFlags: u32 {
        /// Whether embeds are enabled
        const ENABLED = 1 << 0;
        /// Whether the bot should delete old links
        const DELETE_LINK = 1 << 1;
    }
}

impl ToSql for TikTokEmbedFlags {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.bits().into())
    }
}

impl FromSql for TikTokEmbedFlags {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let value = value.as_i64()?;
        let value = u32::try_from(value).map_err(|_e| FromSqlError::OutOfRange(value))?;

        Self::from_bits(value).ok_or_else(|| FromSqlError::OutOfRange(value.into()))
    }
}

impl Default for TikTokEmbedFlags {
    fn default() -> Self {
        Self::empty()
    }
}
