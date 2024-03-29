use crate::{
    checks::ENABLED_CHECK,
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::{
        LoadingReaction,
        TimedCache,
        TimedCacheEntry,
    },
    ClientDataKey,
};
use anyhow::Context as _;
use serenity::{
    builder::{
        CreateEmbed,
        CreateMessage,
    },
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use std::sync::Arc;
use tracing::error;

#[derive(Clone, Debug)]
pub struct SauceNaoClient {
    client: sauce_nao::Client,
    search_cache: TimedCache<String, sauce_nao::OkResponse>,
}

impl SauceNaoClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: sauce_nao::Client::new(api_key),
            search_cache: TimedCache::new(),
        }
    }

    /// Search for an image, with caching
    pub async fn search(
        &self,
        query: &str,
    ) -> anyhow::Result<Arc<TimedCacheEntry<sauce_nao::OkResponse>>> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(entry);
        }

        let search_results = self
            .client
            .search(query)
            .await
            .context("failed to search for image")?;

        self.search_cache
            .insert(String::from(query), search_results);

        self.search_cache
            .get_if_fresh(query)
            .context("cache data expired")
    }
}

impl CacheStatsProvider for SauceNaoClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat(
            "sauce-nao",
            "search_cache",
            self.search_cache.len() as f32,
        );
    }
}

#[command("sauce-nao")]
#[description("Search SauceNao for an image at a url")]
#[usage("<img_url>")]
#[example("https://konachan.com/image/5982d8946ae503351e960f097f84cd90/Konachan.com%20-%20330136%20animal%20nobody%20original%20signed%20yutaka_kana.jpg")]
#[checks(Enabled)]
#[min_args(1)]
#[max_args(1)]
async fn sauce_nao(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let client = client_data.sauce_nao_client.clone();
    drop(data_lock);

    let query = args.trimmed().current().expect("missing query");

    let mut loading = LoadingReaction::new(ctx.http.clone(), msg);

    let result = client
        .search(query)
        .await
        .context("failed to search for image");

    match result {
        Ok(data) => {
            let data = data.data();

            match data.results.first() {
                Some(data) => {
                    let mut embed_builder = CreateEmbed::new()
                        .title("SauceNao Best Match")
                        .image(data.header.thumbnail.as_str());
                    if let Some(ext_url) = data.data.ext_urls.first() {
                        embed_builder = embed_builder
                            .description(ext_url.as_str())
                            .url(ext_url.as_str());
                    }

                    if let Some(source) = data.data.source.as_deref() {
                        embed_builder = embed_builder.field("Source", source, true);
                    }

                    if let Some(eng_name) = data.data.eng_name.as_deref() {
                        embed_builder = embed_builder.field("English Name", eng_name, true);
                    }

                    if let Some(jp_name) = data.data.jp_name.as_deref() {
                        embed_builder = embed_builder.field("Jap Name", jp_name, true);
                    }

                    let message_builder = CreateMessage::new().embed(embed_builder);

                    msg.channel_id
                        .send_message(&ctx.http, message_builder)
                        .await?;

                    loading.send_ok();
                }
                None => {
                    msg.channel_id
                        .say(&ctx.http, format!("No results on SauceNao for \"{query}\""))
                        .await?;
                }
            }
        }
        Err(error) => {
            error!("{error:?}");
            msg.channel_id.say(&ctx.http, format!("{error:?}")).await?;
        }
    }

    Ok(())
}
