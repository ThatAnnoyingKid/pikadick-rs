use crate::{
    util::{
        TimedCache,
        TimedCacheEntry,
    },
    ClientDataKey,
    PoiseContext,
    PoiseError,
};
use anyhow::Context as _;
use poise::reply::CreateReply;
use r6stats::UserData;
use serenity::builder::CreateEmbed;
use std::sync::Arc;
use tracing::{
    error,
    info,
};

#[derive(Clone, Default, Debug)]
pub struct R6StatsClient {
    client: r6stats::Client,
    search_cache: TimedCache<String, UserData>,
}

impl R6StatsClient {
    pub fn new() -> Self {
        Self {
            client: r6stats::Client::new(),
            search_cache: TimedCache::new(),
        }
    }

    /// Get stats
    pub async fn get_stats(
        &self,
        query: &str,
    ) -> Result<Option<Arc<TimedCacheEntry<UserData>>>, r6stats::Error> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(Some(entry));
        }

        let mut user_list = self.client.search(query).await?;

        if user_list.is_empty() {
            return Ok(None);
        }

        let user = user_list.swap_remove(0);

        self.search_cache.insert(String::from(query), user);

        Ok(self.search_cache.get_if_fresh(query))
    }
}

#[poise::command(
    slash_command,
    description_localized("en-US", "Get r6 stats for a user from r6stats"),
    check = "crate::checks::enabled"
)]
pub async fn r6stats(
    ctx: PoiseContext<'_>,
    #[description = "The name of the user"] name: String,
) -> Result<(), PoiseError> {
    let data_lock = ctx.serenity_context().data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let client = client_data.r6stats_client.clone();
    drop(data_lock);

    info!("getting r6 stats for \"{name}\" using r6stats");

    ctx.defer().await?;
    let result = client
        .get_stats(&name)
        .await
        .with_context(|| format!("failed to get stats for \"{name}\" using r6stats"));

    let mut create_reply = CreateReply::default().reply(true);
    match result {
        Ok(Some(entry)) => {
            let data = entry.data();

            let mut embed_builder = CreateEmbed::new();
            embed_builder = embed_builder
                .title(&data.username)
                .image(data.avatar_url_256.as_str());

            if let Some(stats) = data.seasonal_stats.as_ref() {
                embed_builder =
                    embed_builder.field("MMR", ryu::Buffer::new().format(stats.mmr), true);
                embed_builder =
                    embed_builder.field("Max MMR", ryu::Buffer::new().format(stats.max_mmr), true);
                embed_builder = embed_builder.field(
                    "Mean Skill",
                    ryu::Buffer::new().format(stats.skill_mean),
                    true,
                );
            }

            if let Some(kd) = data.kd() {
                embed_builder = embed_builder.field(
                    "Overall Kill / Death",
                    ryu::Buffer::new().format(kd),
                    true,
                );
            }

            if let Some(wl) = data.wl() {
                embed_builder =
                    embed_builder.field("Overall Win / Loss", ryu::Buffer::new().format(wl), true);
            }

            create_reply = create_reply.embed(embed_builder);
        }
        Ok(None) => create_reply = create_reply.content("No results"),
        Err(error) => {
            error!("{error:?}");
            create_reply = create_reply.content(format!("{error:?}"));
        }
    }

    poise::reply::send_reply(ctx, create_reply).await?;

    client.search_cache.trim();

    Ok(())
}
