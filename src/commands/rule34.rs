use crate::{
    Database,
    PoiseContext,
    PoiseError,
    database::{
        Rule34Post,
        Rule34QueryStat,
    },
};
use anyhow::Context as _;
use bewu_util::AsyncMutexMap;
use jiff::SignedDuration;
use poise::{
    CreateReply,
    serenity_prelude::{
        Color,
        CreateEmbed,
        CreateEmbedAuthor,
        CreateEmbedFooter,
    },
};
use rand::{
    RngExt,
    prelude::IndexedRandom,
};
use std::collections::BTreeSet;
use tracing::{
    debug,
    info,
    warn,
};

const ONE_DAY: SignedDuration = SignedDuration::from_hours(24);
const RULE34_ICON_URL: &str = "https://rule34.xxx/apple-touch-icon.png";
const RULE34_COLOR: Color = Color::from_rgb(0xB1, 0xE6, 0xAA);
// Embeds fail to load images sometimes?
const ENABLE_EMBED: bool = false;

fn post_list_to_database_model(post_list: &rule34::PostList) -> Vec<Rule34Post> {
    let last_fetched = jiff::Timestamp::now();
    post_list
        .posts
        .iter()
        .map(|post| Rule34Post {
            id: post.id,
            tags: post.tags.to_string(),
            file_url: post.file_url.to_string(),
            last_fetched,
        })
        .collect()
}

/// A caching rule34 client
#[derive(Debug)]
pub struct Rule34Client {
    client: rule34::Client,
    database: Database,
    search_map: AsyncMutexMap<Option<String>>,
}

impl Rule34Client {
    /// Make a new [`Rule34Client`].
    pub fn new(user_id: u64, api_key: &str, database: Database) -> Rule34Client {
        let client = rule34::Client::new();
        client.set_auth(user_id, api_key);

        let search_map = AsyncMutexMap::new();

        Rule34Client {
            client,
            database,
            search_map,
        }
    }

    /// Search for a query.
    #[tracing::instrument(skip(self))]
    async fn list(&self, query: Option<String>) -> anyhow::Result<rule34::PostList> {
        self.client
            .list_posts()
            .tags(query.as_deref())
            .limit(Some(rule34::POST_LIST_LIMIT_MAX))
            .execute()
            .await
            .context("Failed to search rule34")
    }

    async fn list_posts_and_get_post(&self, query: Option<String>) -> anyhow::Result<Rule34Post> {
        let now = jiff::Timestamp::now();
        let post_list = self.list(query.clone()).await?;
        let posts = post_list_to_database_model(&post_list);
        let chosen_post = posts.choose(&mut rand::rng()).cloned();
        self.database
            .upsert_rule34_posts_and_query_stat(
                posts,
                Rule34QueryStat {
                    query: query.clone().unwrap_or_default(),
                    queries_since_last_fetch: 1,
                    last_fetched: now,
                },
            )
            .await
            .context("Failed to upsert post")?;
        chosen_post.with_context(|| format!("No results for {query:?}"))
    }

    async fn get_random_post_file_url_simple(
        &self,
        query: Option<String>,
    ) -> anyhow::Result<Rule34Post> {
        const HIGH_QUERY_COUNT: u16 = rule34::POST_LIST_LIMIT_MAX / 4;
        const LOW_BUDGET: u8 = rule34::RATELIMIT_BUDGET / 2;

        let random_seed = rand::rng().random::<i64>();
        let limit = 10;
        let (mut posts, query_stat) = self
            .database
            .get_random_rule34_post_and_query_stat(query.clone(), random_seed, Some(limit))
            .await?;

        if query_stat.is_none_or(|value| {
            value.last_fetched.duration_until(jiff::Timestamp::now()) > ONE_DAY
                || value.queries_since_last_fetch > u32::from(HIGH_QUERY_COUNT)
        }) || self.client.ratelimit_budget_remaining() < LOW_BUDGET
            || posts.is_empty()
        {
            debug!("Fetching new data");
            return self.list_posts_and_get_post(query).await;
        }

        // TODO: Handle deleted posts.
        let post = posts
            .pop()
            .with_context(|| format!("No results for {query:?}"))?;

        debug!("Using cached data");

        Ok(post)
    }

    /// Get a random post.
    async fn get_random_post(&self, query: Option<String>) -> anyhow::Result<Rule34Post> {
        let _lock = self.search_map.lock(query.clone()).await;

        // TODO: Expand this check to include AND queries.
        // TODO: Consider using query parser.
        if query
            .as_deref()
            .is_none_or(|value| !value.chars().any(|ch| ch == ' '))
        {
            return self.get_random_post_file_url_simple(query).await;
        }

        self.list_posts_and_get_post(query).await
    }

    /// Autocomplete a tag.
    pub async fn autocomplete(&self, query: &str) -> anyhow::Result<Vec<String>> {
        let results = self.client.autocomplete(query).await?;
        Ok(results.into_iter().map(|result| result.value).collect())
    }
}

async fn autocomplete_query(ctx: PoiseContext<'_>, partial: &str) -> Vec<String> {
    // TODO: Use a full rule34 tag parser here.
    let (head, partial_tag) = partial
        .rsplit_once(' ')
        .map(|(head, tail)| (Some(head), tail))
        .unwrap_or((None, partial));
    let tags = ctx
        .data()
        .rule34_client
        .autocomplete(partial_tag)
        .await
        .context("Failed to autocomplete");
    let tags = match tags {
        Ok(tags) => tags,
        Err(error) => {
            warn!("{error:?}");
            return Vec::new();
        }
    };

    let mut queries = Vec::with_capacity(std::cmp::min(tags.len(), 25));
    for tag in tags.iter() {
        let mut query = String::new();
        if let Some(head) = head.as_ref() {
            query.push_str(head);
            query.push(' ');
        }
        query.push_str(tag);

        queries.push(query);
    }

    queries
}

#[poise::command(
    slash_command,
    description_localized("en-US", "Look up rule34 images from rule34.xxx"),
    check = "crate::checks::enabled"
)]
pub async fn rule34(
    ctx: PoiseContext<'_>,
    #[description = "The rule34.xxx search query. Supports the same syntax as the website."]
    #[autocomplete = "autocomplete_query"]
    query: Option<String>,
) -> Result<(), PoiseError> {
    ctx.defer().await?;

    let query_format = query
        .as_deref()
        .map(|query| format!("{query:?}"))
        .unwrap_or_else(|| String::from("None"));
    info!("Searching rule34 for {query_format}");
    let result = ctx
        .data()
        .rule34_client
        .get_random_post(query.clone())
        .await
        .context("Failed to get search results");

    let mut reply_builder = CreateReply::default().reply(true);
    match result {
        Ok(post) => {
            const MAX_FOOTER_SIZE: usize = 2048;

            let post_url = format!(
                "https://rule34.xxx/index.php?page=post&s=view&id={}",
                post.id
            );
            #[expect(clippy::case_sensitive_file_extension_comparisons)]
            if post.file_url.ends_with(".mp4") || !ENABLE_EMBED {
                let content = format!("[Rule34 {}]({})\n{}", post.id, post_url, post.file_url);
                reply_builder = reply_builder.content(content);
            } else {
                let post_tags: BTreeSet<_> =
                    post.tags.split(' ').filter(|tag| !tag.is_empty()).collect();

                let mut post_tags_string = String::new();
                for tag in post_tags {
                    if post_tags_string.len() + tag.len() + 1 >= MAX_FOOTER_SIZE {
                        break;
                    }

                    if !post_tags_string.is_empty() {
                        post_tags_string.push(' ');
                    }

                    post_tags_string.push_str(tag);
                }

                let author_builder = CreateEmbedAuthor::new("Rule34")
                    .url(post_url)
                    .icon_url(RULE34_ICON_URL);
                let footer = CreateEmbedFooter::new(post.tags);
                let embed_builder = CreateEmbed::default()
                    .title(post.id.to_string())
                    .image(post.file_url)
                    .author(author_builder)
                    .footer(footer)
                    .color(RULE34_COLOR);
                reply_builder = reply_builder.embed(embed_builder);
            }
        }
        Err(error) => {
            warn!("{error:?}");
            reply_builder = reply_builder.content(format!("{error:?}"));
        }
    }
    ctx.send(reply_builder).await?;

    Ok(())
}
