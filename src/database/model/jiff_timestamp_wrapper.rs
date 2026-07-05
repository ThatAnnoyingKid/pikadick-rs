use nd_async_rusqlite::rusqlite::types::{
    FromSql,
    FromSqlError,
    FromSqlResult,
    ToSql,
    ToSqlOutput,
    ValueRef,
};

#[derive(Debug)]
pub struct JiffTimestampWrapper(pub jiff::Timestamp);

impl ToSql for JiffTimestampWrapper {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.0.as_millisecond().into())
    }
}

impl FromSql for JiffTimestampWrapper {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let value = value.as_i64()?;
        let value = jiff::Timestamp::from_millisecond(value)
            .map_err(|error| FromSqlError::Other(error.into()))?;

        Ok(Self(value))
    }
}
