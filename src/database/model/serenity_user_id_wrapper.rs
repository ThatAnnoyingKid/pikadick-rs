use nd_async_rusqlite::rusqlite::types::{
    FromSql,
    FromSqlError,
    FromSqlResult,
    ValueRef,
};
use serenity::model::id::UserId;
use std::num::NonZeroU64;

/// A wrapper for a serenity UserId.
pub struct SerenityUserIdWrapper(pub UserId);

impl FromSql for SerenityUserIdWrapper {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let value_i64 = i64::column_result(value)?;
        let value_u64 = u64::from_ne_bytes(value_i64.to_ne_bytes());
        let value = NonZeroU64::new(value_u64).ok_or(FromSqlError::OutOfRange(0))?;

        Ok(Self(UserId::from(value)))
    }
}
