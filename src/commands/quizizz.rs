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
use std::sync::Arc;
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
    finder_task_rx: WatchReceiver<SearchResult>,
}

impl QuizizzClient {
    /// Make a new [`QuizizzClient`].
    pub fn new() -> Self {
        let finder_task_wakeup = Arc::new(Notify::new());
        let wakeup = finder_task_wakeup.clone();
        let (watch_tx, finder_task_rx): (WatchSender<SearchResult>, WatchReceiver<SearchResult>) =
            tokio::sync::watch::channel(Ok(None));

        tokio::spawn(async move {
            let client = quizizz::Client::new();

            while tokio::select! {
                _ = wakeup.notified() => true,
                _ = watch_tx.closed() => false,
            } {
                let mut code: u32 = rand::random::<u32>() % MAX_CODE;
                let mut tries = 0;

                info!(start_code = code);

                'body: loop {
                    let (tx, mut rx) = tokio::sync::mpsc::channel(10);
                    for _ in 0..10 {
                        let code_str = format!("{:06}", code);

                        code = code.wrapping_add(1);
                        tries += 1;

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

                        if tries == MAX_TRIES {
                            let _ = watch_tx.send(Ok(None)).is_ok();
                            break;
                        }
                    }
                    drop(tx);

                    while let Some((code_str, check_room_result)) = rx.recv().await {
                        match check_room_result {
                            Ok(Some(room)) if room.is_running() => {
                                let _ = watch_tx.send(Ok(Some(code_str))).is_ok();
                                break 'body;
                            }
                            Ok(None) | Ok(Some(_)) => {
                                // Pass
                            }
                            Err(quizizz::Error::InvalidGenericResponse(e))
                                if e.is_room_not_found() || e.is_player_login_required() =>
                            {
                                // Pass
                            }
                            Err(e) => {
                                let e = Err(e)
                                    .with_context(|| {
                                        format!("failed to search for quizizz code '{}'", code_str)
                                    })
                                    .map_err(Arc::new);
                                let _ = watch_tx.send(e).is_ok();
                                break 'body;
                            }
                        }
                    }
                }
            }
        });

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

        // Wake up task if its sleeping
        self.finder_task_wakeup.notify_waiters();

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
