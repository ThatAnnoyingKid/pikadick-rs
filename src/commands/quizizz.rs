use crate::{
    checks::ENABLED_CHECK,
    util::LoadingReaction,
    ClientDataKey,
};
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};

const MAX_TRIES: usize = 1_000;
const MAX_CODE: u32 = 999_999;

#[derive(Default, Clone, Debug)]
pub struct QuizizzClient {
    client: quizizz::Client,
}

impl QuizizzClient {
    pub fn new() -> Self {
        Default::default()
    }
}

#[command]
#[description("Locate a quizizz code")]
#[bucket("quizizz")]
#[checks(Enabled)]
async fn quizizz(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();

    let client = client_data.quizizz_client.clone();

    drop(data_lock);

    let mut loading = LoadingReaction::new(ctx.http.clone(), msg);

    let mut code: u32 = rand::random::<u32>() % MAX_CODE;
    let mut tries = 0;

    loop {
        let code_str = format!("{:06}", code);
        match client.client.check_room(&code_str).await {
            Ok(res) => {
                if res.error.is_none() {
                    if let Some(room) = res.room {
                        if room.is_running() {
                            loading.send_ok();

                            msg.channel_id
                                .say(&ctx.http, format!("Located quizizz code: {}", code_str))
                                .await?;
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!(
                            "Got error while searching for quizizz code '{}', quitting: {}",
                            &code_str, e,
                        ),
                    )
                    .await?;
                break;
            }
        }

        code = code.wrapping_add(1);
        tries += 1;

        if tries == MAX_TRIES {
            msg.channel_id
                .say(
                    &ctx.http,
                    "Reached limit while searching for quizizz code, quitting...",
                )
                .await?;
            break;
        }
    }

    Ok(())
}
