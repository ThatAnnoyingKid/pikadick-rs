use crate::{
    checks::ENABLED_CHECK,
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::LoadingReaction,
    ClientDataKey,
};
use crossbeam::queue::SegQueue;
use fml::{
    types::Article,
    FmlResult,
};
use serenity::{
    client::Context,
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::channel::Message,
};
use std::sync::Arc;
use tracing::error;

/// A caching fml client
#[derive(Clone, Debug)]
pub struct FmlClient {
    client: fml::Client,
    cache: Arc<SegQueue<Article>>,
}

impl FmlClient {
    /// Make a new FmlClient
    pub fn new(key: String) -> Self {
        Self {
            client: fml::Client::new(key),
            cache: Arc::new(SegQueue::new()),
        }
    }

    /// Repopulate the cache
    async fn repopulate(&self) -> FmlResult<()> {
        let articles = self.client.list_random(100).await?;
        for article in articles.into_iter() {
            self.cache.push(article);
        }

        Ok(())
    }

    fn should_repopulate(&self) -> bool {
        self.cache.len() < 50
    }

    fn get_entry(&self) -> Option<Article> {
        self.cache.pop()
    }
}

impl CacheStatsProvider for FmlClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("fml", "cache", self.cache.len() as f32);
    }
}

// TODO: Format command output better
#[command]
#[description("Get a random story from fmylife.com")]
#[checks(Enabled)]
async fn fml(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let client = client_data.fml_client.clone();
    drop(data_lock);

    let mut loading = LoadingReaction::new(ctx.http.clone(), msg);

    if client.should_repopulate() {
        if let Err(e) = client.repopulate().await {
            error!("Failed to repopulate fml cache: {}", e);

            msg.channel_id
                .say(&ctx.http, format!("Failed to get fml entry: {}", e))
                .await?;

            return Ok(());
        }
    }

    if let Some(entry) = client.get_entry() {
        loading.send_ok();

        msg.channel_id
            .send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.title("FML Story");
                    e.description(entry.content_hidden);
                    e.field("I agree, your life sucks", entry.metrics.votes_up, true);
                    e.field("You deserved it", entry.metrics.votes_down, true);

                    e.field("\u{200B}", "\u{200B}", false);

                    e.field(
                        "Reactions",
                        format!(
                            "😐 {}\n\n😃 {}\n\n😲 {}\n\n😂 {}",
                            entry.metrics.smiley_amusing,
                            entry.metrics.smiley_funny,
                            entry.metrics.smiley_weird,
                            entry.metrics.smiley_hilarious
                        ),
                        true,
                    );
                    e
                });

                m
            })
            .await?;
    } else {
        // TODO: Maybe get a lock so this can't fail?
        msg.channel_id
            .say(&ctx.http, "Failed to get fml entry: Cache Empty")
            .await?;
    };

    Ok(())
}
