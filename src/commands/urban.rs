use crate::{
    PoiseContext,
    PoiseError,
};
use anyhow::Context as _;
use bewu_util::AsyncTimedLruCache;
use nd_util::ArcAnyhowError;
use poise::CreateReply;
use serenity::{
    builder::CreateEmbed,
    model::timestamp::Timestamp,
};
use std::{
    sync::Arc,
    time::Duration,
};
use tracing::error;

/// A Caching Urban Dictionary Client
///
#[derive(Debug)]
pub struct UrbanClient {
    client: urban_dictionary::Client,
    search_cache:
        AsyncTimedLruCache<String, Result<Arc<urban_dictionary::DefinitionList>, ArcAnyhowError>>,
}

impl UrbanClient {
    /// Make a new [`UrbanClient`].
    ///
    pub fn new() -> UrbanClient {
        Self {
            client: urban_dictionary::Client::new(),
            search_cache: AsyncTimedLruCache::new(100, Duration::from_secs(5 * 60)),
        }
    }

    /// Get the top result for a query.
    ///
    pub async fn search(
        &self,
        query: String,
    ) -> Result<Arc<urban_dictionary::DefinitionList>, ArcAnyhowError> {
        self.search_cache
            .get(query.clone(), || async move {
                self.client
                    .lookup(&query)
                    .await
                    .context("failed to search urban dictionary")
                    .map(Arc::new)
                    .map_err(ArcAnyhowError::new)
            })
            .await
    }
}

impl Default for UrbanClient {
    fn default() -> Self {
        Self::new()
    }
}

fn populate_embed(
    mut embed_builder: CreateEmbed,
    entry: &urban_dictionary::Definition,
) -> CreateEmbed {
    let mut thumbs_down_buf = itoa::Buffer::new();

    embed_builder = embed_builder
        .title(&entry.word)
        .url(entry.permalink.as_str())
        .field("Definition", entry.get_raw_definition(), false)
        .field("Example", entry.get_raw_example(), false)
        .field("👍", entry.thumbs_up.to_string(), true)
        .field("👎", thumbs_down_buf.format(entry.thumbs_down), true);

    match Timestamp::parse(entry.written_on.as_str()).context("failed to parse timestamp") {
        Ok(timestamp) => {
            embed_builder = embed_builder.timestamp(timestamp);
        }
        Err(error) => {
            error!("{error}");
        }
    }

    embed_builder
}

#[poise::command(
    slash_command,
    description_localized("en-US", "Get the top definition from urbandictionary.com"),
    check = "crate::checks::enabled"
)]
pub async fn urban(
    ctx: PoiseContext<'_>,
    #[description = "The word to look up."] query: String,
) -> Result<(), PoiseError> {
    ctx.defer().await?;

    let result = ctx.data().urban_client.search(query).await;

    let mut create_reply = CreateReply::default();
    match result {
        Ok(result) => {
            if let Some(entry) = result.list.first() {
                let mut embed_builder = CreateEmbed::new();
                embed_builder = populate_embed(embed_builder, entry);
                create_reply = create_reply.embed(embed_builder);
            } else {
                create_reply = create_reply.content("No results");
            }
        }
        Err(error) => {
            error!("{error:?}");
            create_reply = create_reply.content(format!("{error:?}"));
        }
    }
    ctx.send(create_reply.reply(true)).await?;

    Ok(())
}
