use crate::{
    checks::ENABLED_CHECK,
    util::LoadingReaction,
    ClientDataKey,
};
use anyhow::Context as _;
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use std::{
    collections::BinaryHeap,
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};
use tokio::sync::{
    watch::{
        Receiver as WatchReceiver,
        Sender as WatchSender,
    },
    Notify,
};
use tracing::{
    error,
    info,
};

pub type SearchResult = Result<Option<String>, Arc<anyhow::Error>>;

const MAX_TRIES: usize = 1_000;
const MAX_CODE: u32 = 999_999;

const LIMIT_REACHED_MSG: &str = "Reached limit while searching for quizizz code, quitting...";

#[derive(Clone, Debug)]
pub struct QuizizzClient {
    finder_task_wakeup: Arc<Notify>,
    // finder_task_interest: Arc<AtomicU64>,
    finder_task_rx: WatchReceiver<SearchResult>,
}

impl QuizizzClient {
    /// Make a new [`QuizizzClient`].
    pub fn new() -> Self {
        let finder_task_wakeup = Arc::new(Notify::new());
        let wakeup = finder_task_wakeup.clone();
        let (watch_tx, finder_task_rx) = tokio::sync::watch::channel(Ok(None));

        tokio::spawn(finder_task(watch_tx, wakeup));

        Self {
            finder_task_wakeup,
            finder_task_rx,
        }
    }

    /// Get the next searched code.
    ///
    /// `None` signifies that the task ran out of tries.
    pub async fn search_for_code(&self) -> SearchResult {
        let mut finder_task_rx = self.finder_task_rx.clone();

        // Mark current value as seen
        finder_task_rx.borrow_and_update();

        // Wake up task
        //
        // You might be wondering why not `notify_waiters`,
        // as the current code will potentially make the task do an extra unecessary lookup
        // if two requests come in at once.
        // This is because there is an edge case where the task may have sent its response already,
        // but still be processing and caching the other requests when it is woken up.
        // Since it is not waiting, this will make the request task hang until another request wakes up the finder task.
        // It is therefore better to use `notify_one` to ensure the task is always woken up when it needs to be, even spuriously.
        // The finder task is also equipped with a caching mechanism,
        // so spurious wakeups will likely quickly pull a value from there instead of making web requests.
        self.finder_task_wakeup.notify_one();

        // Wait for new value
        finder_task_rx
            .changed()
            .await
            .context("failed to get response from finder task")?;

        // Return new value
        let ret = finder_task_rx.borrow_and_update().clone();
        ret
    }
}

impl Default for QuizizzClient {
    fn default() -> Self {
        Self::new()
    }
}

/// A Cache for quizzizz codes
#[derive(Debug)]
pub struct CodeCache {
    cache: BinaryHeap<(std::cmp::Reverse<Instant>, String)>,
}

impl CodeCache {
    /// Make a new cache
    pub fn new() -> Self {
        // Worst case caches `MAX_TRIES - 1` entries, since we gather MAX_TRIES entries and return one on success.
        Self {
            cache: BinaryHeap::with_capacity(MAX_TRIES),
        }
    }

    /// Get the # of entries
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns true if it is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Trim the cache
    pub fn trim(&mut self) {
        while let Some((time, _)) = self.cache.peek() {
            if time.0.elapsed() > Duration::from_secs(10 * 60) {
                self.cache.pop();
            } else {
                // The newest value has not expired.
                // Exit the peek loop.
                break;
            }
        }
    }

    /// Trim the cache and pop a code if it exists
    pub fn trim_pop(&mut self) -> Option<String> {
        self.trim();
        Some(self.cache.pop()?.1)
    }

    /// Add a code to the cache
    pub fn push(&mut self, code_str: String) {
        self.cache
            .push((std::cmp::Reverse(Instant::now()), code_str));
    }
}

impl Default for CodeCache {
    fn default() -> Self {
        Self::new()
    }
}

async fn finder_task(watch_tx: WatchSender<SearchResult>, wakeup: Arc<Notify>) {
    let client = quizizz::Client::new();
    let mut cache = CodeCache::new();

    while tokio::select! {
        _ = wakeup.notified() => true,
        _ = watch_tx.closed() => false,
    } {
        // Try cache first
        if let Some(code_str) = cache.trim_pop() {
            let _ = watch_tx.send(Ok(Some(code_str))).is_ok();
            continue;
        }

        // Generate start code
        let mut code: u32 = rand::random::<u32>() % MAX_CODE;
        info!(start_code = code);

        // Spawn parallel guesses
        let (tx, mut rx) = tokio::sync::mpsc::channel(MAX_TRIES);
        for _ in 0..MAX_TRIES {
            let code_str = format!("{:06}", code);
            code = code.wrapping_add(1);

            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let check_room_result = client
                    .check_room(&code_str)
                    .await
                    .and_then(|r| r.error_for_response())
                    .map(|res| res.room);

                let _ = tx.send((code_str, check_room_result)).await.is_ok();
            });
        }
        drop(tx);

        // Process parallel guess responses
        let mut sent_response = false;
        while let Some((code_str, check_room_result)) = rx.recv().await {
            match check_room_result {
                Ok(Some(room)) if room.is_running() => {
                    if !sent_response {
                        let _ = watch_tx.send(Ok(Some(code_str))).is_ok();
                        sent_response = true;
                    } else {
                        // Cache extra results
                        cache.push(code_str);
                    }
                }
                Ok(None | Some(_)) => {
                    // Pass
                    // room data is missing / room is not running
                }
                Err(quizizz::Error::InvalidGenericResponse(e))
                    if e.is_room_not_found() || e.is_player_login_required() =>
                {
                    // Pass
                    // the room was not found / the player needs to be logged in to access this game
                }
                Err(e) => {
                    let e = Err(e)
                        .with_context(|| {
                            format!("failed to search for quizizz code '{}'", code_str)
                        })
                        .map_err(Arc::new);
                    error!("{:?}", e);
                    if !sent_response {
                        let _ = watch_tx.send(e).is_ok();
                        sent_response = true;
                    }
                }
            }
        }
        if !sent_response {
            let _ = watch_tx.send(Ok(None)).is_ok();
        }

        info!("quizizz has {} cached entries", cache.len());
    }
}

#[command]
#[description("Locate a quizizz code")]
#[bucket("quizizz")]
#[checks(Enabled)]
async fn quizizz(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("failed to get client data");
    let client = client_data.quizizz_client.clone();
    drop(data_lock);

    let mut loading = LoadingReaction::new(ctx.http.clone(), msg);

    match client.search_for_code().await {
        Ok(Some(code_str)) => {
            info!("located quizizz code '{}'", code_str);
            loading.send_ok();
            msg.channel_id
                .say(&ctx.http, format!("Located quizizz code: {}", code_str))
                .await?;
        }
        Ok(None) => {
            info!("quizziz finder reached limit");
            msg.channel_id.say(&ctx.http, LIMIT_REACHED_MSG).await?;
        }
        Err(e) => {
            error!("{:?}", e);
            msg.channel_id.say(&ctx.http, format!("{:?}", e)).await?;
        }
    }

    Ok(())
}
