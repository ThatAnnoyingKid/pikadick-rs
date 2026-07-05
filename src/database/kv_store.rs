use crate::database::Database;
use anyhow::Context;
use nd_async_rusqlite::rusqlite::{
    OptionalExtension,
    TransactionBehavior,
    named_params,
};

// K/V Store SQL
const GET_STORE_SQL: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/sql/get_store.sql"));
const PUT_STORE_SQL: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/sql/put_store.sql"));

impl Database {
    /// Get a key from the store.
    pub async fn store_get<P, K, V>(&self, prefix: P, key: K) -> anyhow::Result<Option<V>>
    where
        P: AsRef<[u8]>,
        K: AsRef<[u8]>,
        V: serde::de::DeserializeOwned,
    {
        let prefix = prefix.as_ref().to_vec();
        let key = key.as_ref().to_vec();

        let maybe_value: Option<String> = self
            .read(move |database| {
                database
                    .prepare_cached(GET_STORE_SQL)?
                    .query_row(
                        named_params! {
                            ":prefix": prefix,
                            ":key": key,
                        },
                        |row| row.get(0),
                    )
                    .optional()
                    .context("Failed to get value")
            })
            .await??;

        match maybe_value {
            Some(value) => Ok(Some(serde_json::from_str(&value)?)),
            None => Ok(None),
        }
    }

    /// Put a key in the store.
    pub async fn store_put<P, K, V>(&self, prefix: P, key: K, value: &V) -> anyhow::Result<()>
    where
        P: AsRef<[u8]>,
        K: AsRef<[u8]>,
        V: serde::Serialize,
    {
        let prefix = prefix.as_ref().to_vec();
        let key = key.as_ref().to_vec();
        let value = serde_json::to_string(value).context("Failed to serialize value")?;

        self.write(move |database| {
            let transaction = database.transaction()?;
            transaction
                .prepare_cached(PUT_STORE_SQL)?
                .execute(named_params! {
                    ":prefix": prefix,
                    ":key": key,
                    ":value": value
                })?;
            transaction
                .commit()
                .context("Failed to insert key into kv_store")
        })
        .await??;

        Ok(())
    }

    /// Get and Put a key in the store in one action, ensuring the key is not changed between the commands.
    pub async fn store_update<P, K, V, U>(
        &self,
        prefix: P,
        key: K,
        update_func: U,
    ) -> anyhow::Result<()>
    where
        P: AsRef<[u8]>,
        K: AsRef<[u8]>,
        V: serde::Serialize + serde::de::DeserializeOwned,
        U: FnOnce(Option<V>) -> V + Send + 'static,
    {
        let prefix = prefix.as_ref().to_vec();
        let key = key.as_ref().to_vec();

        self.access_db(move |database| {
            let txn = database.transaction_with_behavior(TransactionBehavior::Immediate)?;

            let maybe_value = txn
                .prepare_cached(GET_STORE_SQL)?
                .query_row(
                    named_params! {
                        ":prefix": prefix,
                        ":key": key,
                    },
                    |row| row.get(0),
                )
                .optional()
                .context("Failed to get value")?
                .map(|value: String| {
                    serde_json::from_str(value.as_str()).context("Failed to deserialize value")
                })
                .transpose()?;
            let value = update_func(maybe_value);
            let value = serde_json::to_string(&value).context("Failed to serialize value")?;

            txn.prepare_cached(PUT_STORE_SQL)?.execute(named_params! {
                ":prefix": prefix,
                ":key": key,
                ":value": value,
            })?;
            txn.commit().context("Failed to insert key into kv_store")
        })
        .await??;

        Ok(())
    }
}
