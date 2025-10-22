use crate::database::Database;
use anyhow::Context;
use rusqlite::{
    params,
    OptionalExtension,
    TransactionBehavior,
};

// K/V Store SQL
const GET_STORE_SQL: &str = include_str!("../../sql/get_store.sql");
const PUT_STORE_SQL: &str = include_str!("../../sql/put_store.sql");

impl Database {
    /// Get a key from the store
    pub async fn store_get<P, K, V>(&self, prefix: P, key: K) -> anyhow::Result<Option<V>>
    where
        P: AsRef<[u8]>,
        K: AsRef<[u8]>,
        V: serde::de::DeserializeOwned,
    {
        let prefix = prefix.as_ref().to_vec();
        let key = key.as_ref().to_vec();

        let maybe_bytes: Option<Vec<u8>> = self
            .access_db(move |db| {
                db.prepare_cached(GET_STORE_SQL)?
                    .query_row([prefix, key], |row| row.get(0))
                    .optional()
                    .context("failed to get value")
            })
            .await??;

        match maybe_bytes {
            Some(bytes) => Ok(Some(
                bincode::serde::decode_from_slice(&bytes, bincode::config::legacy())
                    .context("failed to decode value")?
                    .0,
            )),
            None => Ok(None),
        }
    }

    /// Put a key in the store
    pub async fn store_put<P, K, V>(&self, prefix: P, key: K, value: V) -> anyhow::Result<()>
    where
        P: AsRef<[u8]>,
        K: AsRef<[u8]>,
        V: serde::Serialize,
    {
        let prefix = prefix.as_ref().to_vec();
        let key = key.as_ref().to_vec();
        let value = bincode::serde::encode_to_vec(&value, bincode::config::legacy())
            .context("failed to serialize value")?;

        self.access_db(move |db| {
            let txn = db.transaction()?;
            txn.prepare_cached(PUT_STORE_SQL)?
                .execute(params![prefix, key, value])?;
            txn.commit().context("failed to insert key into kv_store")
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

        self.access_db(move |db| {
            let txn = db.transaction_with_behavior(TransactionBehavior::Immediate)?;

            let maybe_value = txn
                .prepare_cached(GET_STORE_SQL)?
                .query_row(params![prefix, key], |row| row.get(0))
                .optional()
                .context("failed to get value")?
                .map(|bytes: Vec<u8>| {
                    bincode::serde::decode_from_slice(&bytes, bincode::config::legacy())
                        .context("failed to decode value")
                        .map(|(value, _)| value)
                })
                .transpose()?;
            let value = update_func(maybe_value);
            let value = bincode::serde::encode_to_vec(&value, bincode::config::legacy())
                .context("failed to serialize value")?;

            txn.prepare_cached(PUT_STORE_SQL)?
                .execute(params![prefix, key, value])?;
            txn.commit().context("failed to insert key into kv_store")
        })
        .await??;

        Ok(())
    }
}
