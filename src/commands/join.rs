use crate::{
    checks::ENABLED_CHECK,
    util::LoadingReaction,
};
use anyhow::Context as _;
use serenity::{
    client::{
        Cache,
        Context,
    },
    framework::standard::{
        macros::command,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use songbird::{
    Call,
    Songbird,
};
use std::sync::Arc;
use tracing::{
    error,
    info,
};

#[derive(Debug, Clone)]
pub struct SongbirdEventHandler {
    cache: Arc<Cache>,
    call: Arc<Mutex<Call>>,
    manager: Arc<Songbird>,
    guild_id: GuildId,
}

impl SongbirdEventHandler {
    pub fn new(
        cache: Arc<Cache>,
        call: Arc<Mutex<Call>>,
        manager: Arc<Songbird>,
        guild_id: GuildId,
    ) -> Self {
        Self {
            cache,
            call,
            manager,
            guild_id,
        }
    }
}

#[serenity::async_trait]
impl songbird::events::EventHandler for SongbirdEventHandler {
    async fn act(&self, ctx: &songbird::EventContext<'_>) -> Option<songbird::Event> {
        if let songbird::EventContext::ClientDisconnect(_data) = ctx {
            let current_channel_id = self.call.lock().await.current_channel();
            let channel = match current_channel_id {
                Some(id) => self.cache.guild_channel(ChannelId(id.0)).await,
                None => None,
            };
            let members = match channel {
                Some(channel) => Some(
                    channel
                        .members(&self.cache)
                        .await
                        .context("failed to retrieve cached members data"),
                ),
                None => None,
            };

            let should_leave = match members {
                Some(Ok(members)) => members.len() == 1,
                Some(Err(e)) => {
                    error!("{:?}", e);
                    true
                }
                None => {
                    error!("missing current channel id");
                    true
                }
            };

            if should_leave {
                info!("Leaving voice channel");
                if let Err(e) = self
                    .manager
                    .remove(self.guild_id)
                    .await
                    .context("failed to leave voice channel")
                {
                    error!("{:?}", e);
                }
            }
        }
        None
    }
}

#[command]
#[only_in(guilds)]
#[bucket("default")]
#[checks(Enabled)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let (guild_id, channel_id) = msg
        .guild_field(&ctx.cache, |guild| {
            (
                guild.id,
                guild
                    .voice_states
                    .get(&msg.author.id)
                    .and_then(|voice_state| voice_state.channel_id),
            )
        })
        .await
        .context("missing server data")?;
    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            msg.channel_id.say(ctx, "Not in a voice channel").await?;
            return Ok(());
        }
    };

    let mut loading_reaction = LoadingReaction::new(ctx.http.clone(), msg);

    let manager = songbird::get(ctx)
        .await
        .expect("missing songbird data")
        .clone();

    let (call, join_result) = manager.join(guild_id, connect_to).await;

    if let Err(e) = join_result.context("failed to join channel") {
        error!("{:?}", e);
        msg.channel_id.say(ctx, format!("{:?}", e)).await?;
        if let Err(e) = call
            .lock()
            .await
            .leave()
            .await
            .context("failed to leave voice channel")
        {
            error!("{:?}", e);
        }
        return Ok(());
    }

    {
        // Handler init/setup
        let mut call_lock = call.lock().await;
        let event_handler =
            SongbirdEventHandler::new(ctx.cache.clone(), call.clone(), manager, guild_id);
        call_lock.add_global_event(
            songbird::Event::Core(songbird::CoreEvent::ClientDisconnect),
            event_handler,
        );
    }

    loading_reaction.send_ok();

    Ok(())
}
