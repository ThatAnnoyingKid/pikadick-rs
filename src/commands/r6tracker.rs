use crate::{
    PoiseContext,
    PoiseError,
};
use anyhow::Context as _;
use bewu_util::AsyncTimedLruCache;
use nd_util::ArcAnyhowError;
use poise::CreateReply;
use serenity::builder::CreateEmbed;
use std::{
    sync::Arc,
    time::Duration,
};
use tracing::{
    error,
    info,
};

const FIVE_MINUTES: Duration = Duration::from_secs(60 * 5);

fn populate_season(
    mut embed_builder: CreateEmbed,
    season: Option<&r6tracker::SegmentSeason>,
    game_mode_name: &str,
) -> CreateEmbed {
    if let Some(season) = season {
        if let Some(name) = season.ranking_rank_name() {
            embed_builder =
                embed_builder.field(format!("Current {game_mode_name} Rank"), name, true);
        }
        if let Some(value) = season.ranking_value() {
            embed_builder = embed_builder.field(
                format!("Current {game_mode_name} MMR"),
                value.to_string(),
                true,
            );
        }

        embed_builder = embed_builder
            .field(
                format!("Seasonal {game_mode_name} K/D"),
                format!("{:.2}", season.kd_ratio_value()),
                true,
            )
            .field(
                format!("Seasonal {game_mode_name} Win %"),
                format!("{:.2}", season.win_percentage_value()),
                true,
            )
            .field(
                format!("Seasonal # of {game_mode_name} Matches"),
                season.matches_played_value().to_string(),
                true,
            )
            .field("", "", false);
    }

    embed_builder
}

/// R6Tracker stats for a user
#[derive(Debug)]
pub struct Stats {
    profile: r6tracker::UserData,
}

impl Stats {
    /// Populate an embed with data.
    pub fn populate_embed(&self, mut embed_builder: CreateEmbed) -> CreateEmbed {
        embed_builder = embed_builder
            .title(self.profile.platform_info.platform_user_handle.as_str())
            .image(self.profile.platform_info.avatar_url.as_str())
            .field(
                "Level",
                self.profile.metadata.clearance_level.to_string(),
                true,
            )
            .field(
                "Suspected Cheater",
                self.profile
                    .user_info
                    .is_suspicious
                    .unwrap_or(false)
                    .to_string(),
                true,
            )
            .field("", "", false);

        let max_ranked_season = self.profile.get_max_ranked_season();
        if let Some(max_ranked_season) = max_ranked_season {
            let max_rank = max_ranked_season.max_ranking_rank_name();
            if let Some(max_rank) = max_rank {
                embed_builder = embed_builder.field("Best Rank", max_rank, true);
            }

            let max_mmr = max_ranked_season.max_ranking_value();
            if let Some(max_mmr) = max_mmr {
                embed_builder = embed_builder.field("Best MMR", max_mmr.to_string(), true);
            }

            embed_builder = embed_builder.field("", "", false);
        }

        let overview = self.profile.get_overview();
        if let Some(overview) = overview {
            embed_builder = embed_builder
                .field(
                    "Lifetime K/D",
                    format!("{:.2}", overview.kd_ratio_value()),
                    true,
                )
                .field(
                    "Lifetime Win %",
                    format!("{:.2}", overview.win_percentage_value()),
                    true,
                )
                .field("", "", false);
        }

        let ranked_game_mode = self.profile.get_ranked_game_mode();
        if let Some(game_mode) = ranked_game_mode {
            embed_builder = embed_builder
                .field(
                    "Lifetime Ranked K/D",
                    format!("{:.2}", game_mode.kd_ratio_value()),
                    true,
                )
                .field(
                    "Lifetime Ranked Win %",
                    format!("{:.2}", game_mode.win_percentage_value()),
                    true,
                )
                .field("", "", false);
        }

        embed_builder = populate_season(
            embed_builder,
            self.profile.get_current_ranked_season(),
            "Ranked",
        );
        embed_builder = populate_season(
            embed_builder,
            self.profile.get_current_casual_season(),
            "Casual",
        );
        embed_builder = populate_season(
            embed_builder,
            self.profile.get_current_unranked_season(),
            "Unranked",
        );

        if let Some(color) = self
            .profile
            .get_current_ranked_season()
            .and_then(|season| season.color_u32())
        {
            embed_builder = embed_builder.color(color);
        }

        if let Some(thumbnail) = self.profile.current_rank_points_image() {
            embed_builder = embed_builder.thumbnail(thumbnail.as_str());
        }

        embed_builder
    }
}

#[derive(Debug)]
pub struct R6TrackerClient {
    client: r6tracker::Client,

    /// The value is `None` if the user could not be found.
    cache: AsyncTimedLruCache<String, Result<Option<Arc<Stats>>, ArcAnyhowError>>,
}

impl R6TrackerClient {
    /// Make a new r6 client with caching.
    pub fn new() -> Self {
        R6TrackerClient {
            client: Default::default(),
            cache: AsyncTimedLruCache::new(100, FIVE_MINUTES),
        }
    }

    /// Get R6Tracker stats for a user.
    pub async fn get_stats(&self, query: &str) -> Result<Option<Arc<Stats>>, ArcAnyhowError> {
        self.cache
            .get(query.to_string(), || async {
                self.client
                    .get_profile(query, r6tracker::Platform::Pc)
                    .await
                    .and_then(|profile_response| match profile_response.into_result() {
                        Ok(profile) => Ok(Some(Arc::new(Stats { profile }))),
                        Err(error) if error.is_missing() => Ok(None),
                        Err(error) => Err(r6tracker::Error::from(error)),
                    })
                    .context("failed to get profile data")
                    .map_err(ArcAnyhowError::new)
            })
            .await
    }
}

impl Default for R6TrackerClient {
    fn default() -> Self {
        Self::new()
    }
}

#[poise::command(
    slash_command,
    description_localized("en-US", "Get r6 stats for a user from r6tracker"),
    check = "crate::checks::enabled"
)]
pub async fn r6tracker(
    ctx: PoiseContext<'_>,
    #[description = "The name of the user"] name: String,
) -> Result<(), PoiseError> {
    info!("Getting r6 stats for \"{name}\" using R6Tracker");

    ctx.defer().await?;
    let result = ctx
        .data()
        .r6tracker_client
        .get_stats(&name)
        .await
        .with_context(|| format!("failed to get r6tracker stats for \"{name}\""));
    let mut create_reply = CreateReply::default();
    match result.as_ref() {
        Ok(Some(stats)) => {
            let embed_builder = stats.populate_embed(CreateEmbed::new());
            create_reply = create_reply.embed(embed_builder);
        }
        Ok(None) => {
            create_reply = create_reply.content("User does not exist or has not played R6 Siege.");
        }
        Err(error) => {
            error!("{error:?}");
            create_reply = create_reply.content(format!("{error:?}"));
        }
    }

    ctx.send(create_reply.reply(true)).await?;

    Ok(())
}
