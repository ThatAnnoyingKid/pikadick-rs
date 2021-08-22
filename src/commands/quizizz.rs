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
use tracing::{
    error,
    info,
};

pub type SearchResult = anyhow::Result<Option<String>>;

const MAX_TRIES: usize = 1_000;
const MAX_CODE: u32 = 999_999;

const LIMIT_REACHED_MSG: &str = "Reached limit while searching for quizizz code, quitting...";

#[derive(Clone, Debug)]
pub struct QuizizzClient {
    finder_task_tx: tokio::sync::mpsc::Sender<tokio::sync::oneshot::Sender<SearchResult>>,
}

impl QuizizzClient {
    /// Make a new [`QuizizzClient`].
    pub fn new() -> Self {
        let (finder_task_tx, mut rx): (
            tokio::sync::mpsc::Sender<tokio::sync::oneshot::Sender<SearchResult>>,
            tokio::sync::mpsc::Receiver<tokio::sync::oneshot::Sender<SearchResult>>,
        ) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let client = quizizz::Client::new();

            while let Some(sender) = rx.recv().await {
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
                            let _ = sender.send(Ok(Some(code_str))).is_ok();
                            break;
                        }
                        Ok(None) | Ok(Some(_)) => {
                            // pass
                        }
                        Err(quizizz::Error::InvalidGenericResponse(e)) if e.is_room_not_found() => {
                            // Pass
                        }
                        Err(e) => {
                            let _ = sender
                                .send(Err(e).with_context(|| {
                                    format!("failed to search for quizizz code '{}'", code_str)
                                }))
                                .is_ok();
                            break;
                        }
                    }

                    code = code.wrapping_add(1);
                    tries += 1;

                    if tries == MAX_TRIES {
                        let _ = sender.send(Ok(None)).is_ok();
                        break;
                    }
                }
            }
        });

        Self { finder_task_tx }
    }

    /// Get the next searched code
    pub async fn search_for_code(&self) -> SearchResult {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.finder_task_tx
            .send(tx)
            .await
            .context("finder task died")?;
        rx.await
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
            loading.send_ok();
            msg.channel_id
                .say(&ctx.http, format!("Located quizizz code: {}", code_str))
                .await?;
        }
        Ok(None) => {
            msg.channel_id.say(&ctx.http, LIMIT_REACHED_MSG).await?;
            info!("quizziz finder reached limit");
        }
        Err(e) => {
            msg.channel_id.say(&ctx.http, format!("{:?}", e)).await?;
            error!("{:?}", e);
        }
    }

    Ok(())
}
