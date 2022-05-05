use futures::future::{
    BoxFuture,
    FutureExt,
    Shared,
};
use std::{
    collections::{
        hash_map::Entry,
        HashMap,
    },
    fmt::Debug,
    future::Future,
    hash::Hash,
};
use tracing::error;

/// A type to prevent two async requests from racing the same resource.
#[derive(Debug)]
pub struct RequestMap<K, V> {
    map: parking_lot::Mutex<HashMap<K, Shared<BoxFuture<'static, V>>>>,
}

impl<K, V> RequestMap<K, V> {
    /// Make a new [`RequestMap`]
    pub fn new() -> Self {
        Self {
            map: parking_lot::Mutex::new(HashMap::new()),
        }
    }
}

impl<K, V> RequestMap<K, V>
where
    K: Eq + Hash + Clone + Debug,
    V: Clone,
{
    /// Lock the key if it is missing, or run a future to fetch the resource
    pub async fn get_or_fetch<FN, F>(&self, key: K, fetch_future_func: FN) -> V
    where
        FN: FnOnce() -> F,
        F: Future<Output = V> + Send + 'static,
    {
        let (_maybe_guard, shared_future) = {
            // Lock the map
            let mut map = self.map.lock();

            // Get the entry
            match map.entry(key.clone()) {
                Entry::Occupied(entry) => {
                    // A request is already in progress.

                    // Grab the response future and await it.
                    // Don't return a drop guard; only the task that started the request is allowed to clean it up.
                    (None, entry.get().clone())
                }
                Entry::Vacant(entry) => {
                    // A request is not in progress.

                    // First, make the future.
                    let fetch_future = fetch_future_func();

                    // Then, make that future sharable.
                    let shared_future = fetch_future.boxed().shared();

                    // Then, store a copy in the hashmap for others interested in this value.
                    entry.insert(shared_future.clone());

                    // Then, register a drop guard since we own this request,
                    // and are therefore responsible for cleaning it up.
                    let drop_guard = RequestMapDropGuard { key, map: self };

                    // Finally, return the future so we can await it in the next step.
                    (Some(drop_guard), shared_future)
                }
            }
        };

        shared_future.await
    }
}

impl<K, V> Default for RequestMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// This will remove an entry from the request map when it gets dropped
struct RequestMapDropGuard<'a, K, V>
where
    K: Eq + Hash + Debug,
{
    key: K,
    map: &'a RequestMap<K, V>,
}

impl<K, V> Drop for RequestMapDropGuard<'_, K, V>
where
    K: Eq + Hash + Debug,
{
    fn drop(&mut self) {
        // Remove the key from the request map as we are done downloading it.
        if self.map.map.lock().remove(&self.key).is_none() {
            // Normally, a panic would be good,
            // as somebody cleaned up something they didn't own.
            // However, this is a destructor, and a panic here could easily abort.
            // Instead, we will log an error in the console.
            error!("key `{:?}` was unexpectedly cleaned up", self.key);
        }
    }
}
