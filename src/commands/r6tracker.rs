use crate::{
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::{
        TimedCache,
        TimedCacheEntry,
    },
    ClientDataKey,
};
use anyhow::Context as _;
use serenity::builder::{
    CreateEmbed,
    EditInteractionResponse,
};
use std::sync::Arc;
use tracing::{
    error,
    info,
};

/// R6Tracker stats for a user
#[derive(Debug)]
pub struct Stats {
    overwolf_player: r6tracker::OverwolfPlayer,
    profile: r6tracker::UserData,
}

impl Stats {
    /// Populate an embed with data.
    pub fn populate_embed(&self, mut embed_builder: CreateEmbed) -> CreateEmbed {
        // We mix overwolf and non-overwolf data to get what we want.

        // Overwolf Api
        embed_builder = embed_builder
            .title(self.overwolf_player.name.as_str())
            .image(self.overwolf_player.avatar.as_str())
            .field("Level", self.overwolf_player.level.to_string(), true)
            .field(
                "Suspected Cheater",
                self.overwolf_player.suspected_cheater.to_string(),
                true,
            );

        if let Some(season) = self.overwolf_player.current_season_best_region.as_ref() {
            embed_builder = embed_builder
                .field("Current Rank", &season.rank_name, true)
                .field("Current MMR", season.mmr.to_string(), true)
                .field("Seasonal Ranked K/D", format!("{:.2}", season.kd), true)
                .field(
                    "Seasonal Ranked Win %",
                    format!("{:.2}", season.win_pct),
                    true,
                )
                .field(
                    "Seasonal # of Ranked Matches",
                    season.matches.to_string(),
                    true,
                );
        }

        if let Some(season) = self.overwolf_player.get_current_casual_season() {
            embed_builder = embed_builder
                .field("Current Casual Rank", &season.rank_name, true)
                .field("Current Casual MMR", season.mmr.to_string(), true)
                .field("Seasonal Casual K/D", format!("{:.2}", season.kd), true)
                .field(
                    "Seasonal Casual Win %",
                    format!("{:.2}", season.win_pct),
                    true,
                )
                .field(
                    "Seasonal # of Casual Matches",
                    season.matches.to_string(),
                    true,
                );
        }

        // Best Rank/MMR lifetime stats are bugged in Overwolf.
        // It shows the max ending stats.
        //
        // Try manual calculation based on the Overwolf API Season stats,
        // falling back to manual calculation based on the overlay stats,
        // falling back to the Overwolf API lifetime value, which is bugged.
        let max_overwolf_season = self.overwolf_player.get_max_season();
        let max_season = self.profile.get_max_season();
        let overwolf_best_mmr = self.overwolf_player.lifetime_stats.best_mmr.as_ref();
        let max_mmr = max_overwolf_season
            .map(|season| season.max_mmr)
            .or_else(|| max_season.and_then(|season| season.max_mmr()))
            .or_else(|| overwolf_best_mmr.map(|best_mmr| best_mmr.mmr));
        let max_rank = max_overwolf_season
            .map(|season| season.max_rank.rank_name.as_str())
            .or_else(|| {
                max_season
                    .and_then(|season| season.max_rank())
                    .map(|rank| rank.name())
            })
            .or_else(|| overwolf_best_mmr.map(|best_mmr| best_mmr.name.as_str()));

        if let Some(max_mmr) = max_mmr {
            embed_builder = embed_builder.field("Best MMR", max_mmr.to_string(), true);
        }

        if let Some(max_rank) = max_rank {
            embed_builder = embed_builder.field("Best Rank", max_rank, true);
        }

        if let Some(lifetime_ranked_kd) = self.overwolf_player.get_lifetime_ranked_kd() {
            embed_builder = embed_builder.field(
                "Lifetime Ranked K/D",
                format!("{lifetime_ranked_kd:.2}"),
                true,
            );
        }

        if let Some(lifetime_ranked_win_pct) = self.overwolf_player.get_lifetime_ranked_win_pct() {
            embed_builder = embed_builder.field(
                "Lifetime Ranked Win %",
                format!("{lifetime_ranked_win_pct:.2}"),
                true,
            );
        }

        embed_builder = embed_builder
            .field(
                "Lifetime K/D",
                format!("{:.2}", self.overwolf_player.lifetime_stats.kd),
                true,
            )
            .field(
                "Lifetime Win %",
                format!("{:.2}", self.overwolf_player.lifetime_stats.win_pct),
                true,
            );

        // Old Non-Overwolf API

        // Overwolf API does not send season colors
        if let Some(c) = self.profile.season_color_u32() {
            embed_builder = embed_builder.color(c);
        }

        // Overwolf API does not send non-svg rank thumbnails
        if let Some(thumb) = self.profile.current_mmr_image() {
            embed_builder = embed_builder.thumbnail(thumb.as_str());
        }

        embed_builder
    }
}

#[derive(Clone, Default, Debug)]
pub struct R6TrackerClient {
    client: r6tracker::Client,
    /// The value is `None` if the user could not be found
    search_cache: TimedCache<String, Option<Stats>>,
}

impl R6TrackerClient {
    /// Make a new r6 client with caching
    pub fn new() -> Self {
        R6TrackerClient {
            client: Default::default(),
            search_cache: Default::default(),
        }
    }

    /// Get R6Tracker stats
    pub async fn get_stats(
        &self,
        query: &str,
    ) -> anyhow::Result<Arc<TimedCacheEntry<Option<Stats>>>> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(entry);
        }

        let overwolf_client = self.client.clone();
        let overwolf_query = query.to_string();
        let overwolf_player_handle =
            tokio::spawn(async move { overwolf_client.get_overwolf_player(&overwolf_query).await });

        let profile_client = self.client.clone();
        let profile_query = query.to_string();
        let profile_handle = tokio::spawn(async move {
            profile_client
                .get_profile(&profile_query, r6tracker::Platform::Pc)
                .await
        });

        let overwolf_player = overwolf_player_handle.await?;
        let profile = profile_handle.await?;

        // This returns "No results" to the user when an InvalidName Overwolf API Error occurs.
        // This works because we check for errors in the Overwolf response first,
        // so non-existent users are always predictably caught there.
        //
        // However, it may be beneficial to add a case for other API errors to catch edge cases,
        // such as UserData erroring while Overwolf.
        // This isn't a high priortiy however as this is entirely cosmetic;
        // the user will simply get an ugly error if we fail to special-case it here.
        //
        // TODO: Add case for UserData
        let overwolf_player = match overwolf_player
            .context("failed to get overwolf player data")?
            .into_result()
        {
            Ok(overwolf_player) => Some(overwolf_player),
            Err(response_err) if response_err.0.as_str() == "InvalidName" => None,
            Err(e) => {
                return Err(r6tracker::Error::from(e)).context("overwolf api response error");
            }
        };

        // Open profile in the map so that we only validate the profile if we got overwolf data
        // This is because profile will fail in strange ways if the player does not exist.
        let entry = overwolf_player
            .map(|overwolf_player| {
                let profile = profile
                    .context("failed to get profile data")?
                    .into_result()
                    .context("profile api response was invalid")?;
                anyhow::Ok(Stats {
                    overwolf_player,
                    profile,
                })
            })
            .transpose()?;

        self.search_cache.insert(String::from(query), entry);

        self.search_cache
            .get_if_fresh(query)
            .context("cache data expired")
    }
}

impl CacheStatsProvider for R6TrackerClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat(
            "r6tracker",
            "search_cache",
            self.search_cache.len() as f32,
        );
    }
}

/// Options for r6tracker
#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct R6TrackerOptions {
    /// The user name
    name: String,
}

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command> {
    pikadick_slash_framework::CommandBuilder::new()
        .name("r6tracker")
        .description("Get r6 stats for a user from r6tracker")
        .argument(
            pikadick_slash_framework::ArgumentParamBuilder::new()
                .name("name")
                .description("The name of the user")
                .kind(pikadick_slash_framework::ArgumentKind::String)
                .required(true)
                .build()?,
        )
        .on_process(|ctx, interaction, args: R6TrackerOptions| async move {
            let data_lock = ctx.data.read().await;
            let client_data = data_lock
                .get::<ClientDataKey>()
                .expect("missing client data");
            let client = client_data.r6tracker_client.clone();
            drop(data_lock);

            let name = args.name;

            info!("Getting r6 stats for \"{name}\" using R6Tracker");

            interaction.defer(&ctx.http).await?;

            let result = client
                .get_stats(&name)
                .await
                .with_context(|| format!("failed to get r6tracker stats for \"{name}\""));

            let mut edit_response_builder = EditInteractionResponse::new();
            match result.as_ref().map(|entry| entry.data()) {
                Ok(Some(stats)) => {
                    let embed_builder = stats.populate_embed(CreateEmbed::new());
                    edit_response_builder = edit_response_builder.embed(embed_builder);
                }
                Ok(None) => {
                    edit_response_builder = edit_response_builder.content("No Results");
                }
                Err(error) => {
                    error!("{error:?}");
                    edit_response_builder = edit_response_builder.content(format!("{error:?}"));
                }
            }

            interaction
                .edit_response(&ctx.http, edit_response_builder)
                .await?;

            client.search_cache.trim();

            Ok(())
        })
        .build()
        .context("failed to build r6tracker command")
}
