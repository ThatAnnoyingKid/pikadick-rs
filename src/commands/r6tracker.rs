use crate::{
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::{
        TimedCache,
        TimedCacheEntry,
    },
    BotContext,
};
use anyhow::Context as _;
use pikadick_slash_framework::ClientData;
use std::sync::Arc;
use tracing::{
    error,
    info,
};
use twilight_model::{
    channel::embed::Embed,
    http::interaction::{
        InteractionResponse,
        InteractionResponseType,
    },
};
use twilight_util::builder::{
    embed::{
        EmbedBuilder,
        EmbedFieldBuilder,
        ImageSource,
    },
    InteractionResponseDataBuilder,
};

/// R6Tracker stats for a user
#[derive(Debug)]
pub struct Stats {
    overwolf_player: r6tracker::OverwolfPlayer,
    profile: r6tracker::UserData,
}

impl Stats {
    /// Create an embed
    pub fn create_embed(&self) -> anyhow::Result<Embed> {
        let mut embed_builder = EmbedBuilder::new();
        // We mix overwolf and non-overwolf data to get what we want.

        // New Overwolf Api
        embed_builder = embed_builder
            .title(self.overwolf_player.name.as_str())
            .image(ImageSource::url(self.overwolf_player.avatar.as_str())?);

        if let Some(season) = self.overwolf_player.current_season_best_region.as_ref() {
            embed_builder = embed_builder
                .field(EmbedFieldBuilder::new("Current Rank", &season.rank_name).inline())
                .field(
                    EmbedFieldBuilder::new("Current MMR", itoa::Buffer::new().format(season.mmr))
                        .inline(),
                )
                .field(
                    EmbedFieldBuilder::new("Seasonal Ranked K/D", format!("{:.2}", season.kd))
                        .inline(),
                )
                .field(
                    EmbedFieldBuilder::new(
                        "Seasonal Ranked Win %",
                        ryu::Buffer::new().format(season.win_pct),
                    )
                    .inline(),
                )
                .field(
                    EmbedFieldBuilder::new(
                        "Seasonal # of Ranked Matches",
                        itoa::Buffer::new().format(season.matches),
                    )
                    .inline(),
                );
        }

        if let Some(season) = self.overwolf_player.get_current_casual_season() {
            embed_builder = embed_builder
                .field(EmbedFieldBuilder::new("Current Casual Rank", &season.rank_name).inline())
                .field(
                    EmbedFieldBuilder::new(
                        "Current Casual MMR",
                        itoa::Buffer::new().format(season.mmr),
                    )
                    .inline(),
                )
                .field(
                    EmbedFieldBuilder::new("Seasonal Casual K/D", format!("{:.2}", season.kd))
                        .inline(),
                )
                .field(
                    EmbedFieldBuilder::new(
                        "Seasonal Casual Win %",
                        ryu::Buffer::new().format(season.win_pct),
                    )
                    .inline(),
                )
                .field(
                    EmbedFieldBuilder::new(
                        "Seasonal # of Casual Matches",
                        itoa::Buffer::new().format(season.matches),
                    )
                    .inline(),
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
            embed_builder = embed_builder.field(
                EmbedFieldBuilder::new("Best MMR", itoa::Buffer::new().format(max_mmr)).inline(),
            );
        }

        if let Some(max_rank) = max_rank {
            embed_builder =
                embed_builder.field(EmbedFieldBuilder::new("Best Rank", max_rank).inline());
        }

        if let Some(lifetime_ranked_kd) = self.overwolf_player.get_lifetime_ranked_kd() {
            embed_builder = embed_builder.field(
                EmbedFieldBuilder::new("Lifetime Ranked K/D", format!("{:.2}", lifetime_ranked_kd))
                    .inline(),
            );
        }

        if let Some(lifetime_ranked_win_pct) = self.overwolf_player.get_lifetime_ranked_win_pct() {
            embed_builder = embed_builder.field(
                EmbedFieldBuilder::new(
                    "Lifetime Ranked Win %",
                    format!("{:.2}", lifetime_ranked_win_pct),
                )
                .inline(),
            );
        }

        embed_builder = embed_builder
            .field(
                EmbedFieldBuilder::new(
                    "Lifetime K/D",
                    ryu::Buffer::new().format(self.overwolf_player.lifetime_stats.kd),
                )
                .inline(),
            )
            .field(
                EmbedFieldBuilder::new(
                    "Lifetime Win %",
                    ryu::Buffer::new().format(self.overwolf_player.lifetime_stats.win_pct),
                )
                .inline(),
            );

        // Old Non-Overwolf API

        // Overwolf API does not send season colors
        if let Some(c) = self.profile.season_color_u32() {
            embed_builder = embed_builder.color(c);
        }

        // Overwolf API does not send non-svg rank thumbnails
        if let Some(thumb) = self.profile.current_mmr_image() {
            embed_builder = embed_builder.thumbnail(ImageSource::url(thumb.as_str())?);
        }

        Ok(embed_builder.build())
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
            .map::<Result<_, anyhow::Error>, _>(|overwolf_player| {
                let profile = profile
                    .context("failed to get profile data")?
                    .into_result()
                    .context("profile api response was invalid")?;
                Ok(Stats {
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
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
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
        .on_process(
            |client_data, interaction, args: R6TrackerOptions| async move {
                let client = client_data.inner.r6tracker_client.clone();
                let name = args.name.as_str();
                info!("getting r6 stats for '{name}' using r6tracker");

                let result = client
                    .get_stats(name)
                    .await
                    .with_context(|| format!("failed to get r6tracker stats for '{name}'"));

                let interaction_client = client_data.interaction_client();
                let mut response_data = InteractionResponseDataBuilder::new();
                match result.map(|entry| entry.data().as_ref().map(|stats| stats.create_embed())) {
                    Ok(Some(Ok(embed))) => {
                        response_data = response_data.embeds(std::iter::once(embed));
                    }
                    Ok(None) => {
                        response_data = response_data.content("No Results");
                    }
                    Err(e) | Ok(Some(Err(e))) => {
                        error!("{e:?}");
                        response_data = response_data.content(format!("{e:?}"));
                    }
                }
                let response_data = response_data.build();
                let response = InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(response_data),
                };

                interaction_client
                    .create_response(interaction.id, &interaction.token, &response)
                    .exec()
                    .await?;

                client.search_cache.trim();

                Ok(())
            },
        )
        .build()
        .context("failed to build r6tracker command")
}
