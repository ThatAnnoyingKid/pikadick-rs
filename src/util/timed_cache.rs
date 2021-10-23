use dashmap::DashMap;
use parking_lot::Mutex;
use rand::seq::IteratorRandom;
use std::{
    borrow::Borrow,
    hash::Hash,
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};

/// 10 minutes
const DEFAULT_EXPIRE_TIME: Duration = Duration::from_secs(10 * 60);

/// A cache with entries that "expire" after a per-cache time limit
pub struct TimedCache<K, V>(Arc<TimedCacheInner<K, V>>);

struct TimedCacheInner<K, V> {
    cache: DashMap<K, Arc<TimedCacheEntry<V>>>,
    last_trim: Mutex<Instant>,

    trim_time: Duration,
    expiry_time: Duration,
}

impl<K, V> TimedCache<K, V>
where
    K: Eq + Hash + 'static,
    V: 'static,
{
    /// Create a cache with timed entries with a default expire time
    pub fn new() -> Self {
        TimedCache(Arc::new(TimedCacheInner {
            cache: DashMap::new(),
            last_trim: Mutex::new(Instant::now()),

            trim_time: DEFAULT_EXPIRE_TIME,
            expiry_time: DEFAULT_EXPIRE_TIME,
        }))
    }

    /// Get a value if fresh, or None if it doesn't exist or is expired
    pub fn get_if_fresh<Q>(&self, key: &Q) -> Option<Arc<TimedCacheEntry<V>>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.0.cache.get(key).and_then(|entry| {
            if entry.is_fresh(self.0.expiry_time) {
                Some(entry.value().clone())
            } else {
                None
            }
        })
    }

    /// Get a random fresh value
    pub fn get_random_if_fresh(&self) -> Option<Arc<TimedCacheEntry<V>>> {
        self.0
            .cache
            .iter()
            .filter(|entry| entry.is_fresh(self.0.expiry_time))
            .choose(&mut rand::thread_rng())
            .map(|v| v.value().clone())
    }

    /// Insert a K/V
    pub fn insert(&self, key: K, value: V) {
        self.0.cache.insert(
            key,
            Arc::new(TimedCacheEntry {
                data: value,
                last_update: Instant::now(),
            }),
        );
    }

    /// Insert a K/V and return the data for the newly inserted value
    pub fn insert_and_get(&self, key: K, value: V) -> Arc<TimedCacheEntry<V>> {
        let data = Arc::new(TimedCacheEntry {
            data: value,
            last_update: Instant::now(),
        });
        self.0.cache.insert(key, data.clone());
        data
    }

    /// Trims expired entries
    pub fn trim(&self) -> bool {
        let mut last_trim = self.0.last_trim.lock();
        if Instant::now().duration_since(*last_trim) > self.0.trim_time {
            *last_trim = Instant::now();
            drop(last_trim);
            self.force_trim();

            true
        } else {
            false
        }
    }

    /// Trims expired entries, ignoring last trim time.
    pub fn force_trim(&self) {
        let expiry_time = self.0.expiry_time;
        self.0.cache.retain(|_, v| !v.is_fresh(expiry_time));
    }

    /// Gets the number of entries. Includes expired entries.
    pub fn len(&self) -> usize {
        self.0.cache.len()
    }

    /// Checks if cache is empty. Included expired entries.
    pub fn is_empty(&self) -> bool {
        self.0.cache.is_empty()
    }
}

impl<K, V> Default for TimedCache<K, V>
where
    K: Eq + Hash + 'static,
    V: 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Clone for TimedCache<K, V>
where
    K: Eq + Hash + 'static,
    V: 'static,
{
    fn clone(&self) -> Self {
        TimedCache(self.0.clone())
    }
}

impl<K, V> std::fmt::Debug for TimedCache<K, V>
where
    K: Eq + std::fmt::Debug + Hash,
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimedCache")
            .field("cache", &self.0.cache)
            .finish()
    }
}

#[derive(Debug)]
pub struct TimedCacheEntry<T> {
    data: T,
    last_update: Instant,
}

impl<T> TimedCacheEntry<T> {
    /// time is expire time
    pub fn is_fresh(&self, time: Duration) -> bool {
        self.last_update.elapsed() < time
    }

    /// Get data ref
    pub fn data(&self) -> &T {
        &self.data
    }
}
