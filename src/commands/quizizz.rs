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
    mpsc::{
        Receiver as MpscReceiver,
        Sender as MpscSender,
    },
    watch::{
        Receiver as WatchReceiver,
        Sender as WatchSender,
    },
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
    finder_task_tx: MpscSender<()>,

    finder_task_rx: WatchReceiver<SearchResult>,
}

impl QuizizzClient {
    /// Make a new [`QuizizzClient`].
    pub fn new() -> Self {
        let (finder_task_tx, mut rx): (MpscSender<()>, MpscReceiver<()>) =
            tokio::sync::mpsc::channel(100);
        let (watch_tx, finder_task_rx): (WatchSender<SearchResult>, WatchReceiver<SearchResult>) =
            tokio::sync::watch::channel(Ok(None));

        tokio::spawn(async move {
            let client = quizizz::Client::new();

            while let Some(()) = rx.recv().await {
                let mut code: u32 = rand::random::<u32>() % MAX_CODE;
                let mut tries = 0;

                loop {
                    let code_str = format!("{:06}", code);
                    let check_room_result = client
                        .check_room(&code_str)
                        .await
                        .and_then(|r| r.error_for_response());

                    match check_room_result.map(|res| res.room) {
                        Ok(Some(room)) if room.is_running() => {
                            let _ = watch_tx.send(Ok(Some(code_str))).is_ok();
                            break;
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
                            break;
                        }
                    }

                    code = code.wrapping_add(1);
                    tries += 1;

                    if tries == MAX_TRIES {
                        let _ = watch_tx.send(Ok(None)).is_ok();
                        break;
                    }
                }
            }
        });

        Self {
            finder_task_tx,
            finder_task_rx,
        }
    }

    /// Get the next searched code.
    ///
    /// `None` signifies that the task ran out of tries.
    pub async fn search_for_code(&self) -> SearchResult {
        self.finder_task_tx
            .send(())
            .await
            .context("finder task died")?;
        let mut finder_task_rx = self.finder_task_rx.clone();
        finder_task_rx
            .changed()
            .await
            .map(|_| finder_task_rx.borrow().clone())
            .context("failed to get response from finder task")?
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
