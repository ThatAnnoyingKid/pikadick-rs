use crate::{
    checks::ENABLED_CHECK,
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::TimedCache,
    ClientDataKey,
};
use rand::prelude::IndexedRandom;
use serenity::{
    framework::standard::{
        macros::command,
        ArgError,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use shift_orcz::{
    Client as OrczClient,
    Game,
    ShiftCode,
};
use std::{
    str::FromStr,
    sync::Arc,
};

#[derive(Debug)]
struct GameParseError(String);

struct GameArg(Game);

impl FromStr for GameArg {
    type Err = GameParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bl" => Ok(Self(Game::Borderlands)),
            "bl2" => Ok(Self(Game::Borderlands2)),
            "blps" => Ok(Self(Game::BorderlandsPreSequel)),
            "bl3" => Ok(Self(Game::Borderlands3)),
            _ => Err(GameParseError(s.into())),
        }
    }
}

#[derive(Default, Clone)]
pub struct ShiftClient {
    orcz_client: OrczClient,
    cache: TimedCache<Game, Vec<Arc<ShiftCode>>>,
}

impl ShiftClient {
    pub fn new() -> Self {
        ShiftClient {
            orcz_client: OrczClient::new(),
            cache: TimedCache::new(),
        }
    }

    /// Get a random shift code (PC only for now...)
    pub async fn get_rand(
        &self,
        game: Game,
    ) -> Result<Option<Arc<ShiftCode>>, shift_orcz::OrczError> {
        if let Some(entry) = self.cache.get_if_fresh(&game) {
            return Ok(entry.data().choose(&mut rand::thread_rng()).cloned());
        }

        let codes = self
            .orcz_client
            .get_shift_codes(game)
            .await?
            .into_iter()
            .filter(|e| e.pc.is_valid())
            .map(Arc::new)
            .collect();

        self.cache.insert(game, codes);

        Ok(self
            .cache
            .get_if_fresh(&game)
            .and_then(|entry| entry.data().choose(&mut rand::thread_rng()).cloned()))
    }
}

impl CacheStatsProvider for ShiftClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat(
            "shift",
            "bl_cache",
            self.cache
                .get_if_fresh(&Game::Borderlands)
                .map(|el| el.data().len())
                .unwrap_or(0) as f32,
        );

        cache_stats_builder.publish_stat(
            "shift",
            "bl2_cache",
            self.cache
                .get_if_fresh(&Game::Borderlands2)
                .map(|el| el.data().len())
                .unwrap_or(0) as f32,
        );

        cache_stats_builder.publish_stat(
            "shift",
            "blps_cache",
            self.cache
                .get_if_fresh(&Game::BorderlandsPreSequel)
                .map(|el| el.data().len())
                .unwrap_or(0) as f32,
        );

        cache_stats_builder.publish_stat(
            "shift",
            "bl3_cache",
            self.cache
                .get_if_fresh(&Game::Borderlands3)
                .map(|el| el.data().len())
                .unwrap_or(0) as f32,
        );
    }
}

impl std::fmt::Debug for ShiftClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: replace with derive impl if/when orcz_client becomes Debug-able
        f.debug_struct("ShiftClient")
            .field("cache", &self.cache)
            .finish()
    }
}

#[command]
#[description("Get a random shift code for a Borderlands game")]
#[min_args(1)]
#[usage("<bl, bl2, blps, or bl3>")]
#[example("blps")]
#[checks(Enabled)]
#[bucket("default")]
async fn shift(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let client = client_data.shift_client.clone();
    drop(data_lock);

    let game = match args.single::<GameArg>().map(|el| el.0) {
        Ok(g) => g,
        Err(ArgError::Parse(e)) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Invalid arg '{}'. Valid: bl, bl2, blps, bl3", e.0),
                )
                .await?;
            return Ok(());
        }
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Need arg. Valid: bl, bl2, blps, bl3")
                .await?;
            return Ok(());
        }
    };

    match client.get_rand(game).await {
        Ok(Some(code)) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!(
                        "Source: {}\nIssue Date: {}\nReward: {}\nCode: {}",
                        code.source,
                        code.issue_date
                            .map(|d| d.to_string())
                            .unwrap_or_else(|| "unknown".to_string()),
                        code.rewards,
                        code.pc
                    ),
                )
                .await?;
        }
        Ok(None) => {
            msg.channel_id
                .say(&ctx.http, format!("No valid codes for {:?}", game))
                .await?;
        }
        Err(e) => {
            msg.channel_id
                .say(&ctx.http, format!("Failed to get shift code: {:#?}", e))
                .await?;
        }
    }

    client.cache.trim();

    Ok(())
}
