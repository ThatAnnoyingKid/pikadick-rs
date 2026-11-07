mod stats;
mod user;

use self::{
    stats::stats,
    user::{
        UserStatsRenderer,
        user,
    },
};
use crate::{
    PoiseContext,
    PoiseError,
};
use anyhow::{
    Context,
    ensure,
};
use bewu_util::AsyncMutexMap;
use camino::{
    Utf8Path,
    Utf8PathBuf,
};
use poise::serenity_prelude::CreateAutocompleteResponse;
use std::{
    sync::Arc,
    time::Instant,
};
use tokio::io::AsyncWriteExt;
use tracing::{
    info,
    warn,
};

#[derive(Debug)]
pub struct UggClient {
    client: ugg::Client,
    icon_lock_map: AsyncMutexMap<u32>,
    profile_icons_dir: Utf8PathBuf,
    rank_icon_lock_map: AsyncMutexMap<String>,
    rank_icons_dir: Utf8PathBuf,
    user_stats_renderer: Arc<tokio::sync::Mutex<UserStatsRenderer>>,
}

impl UggClient {
    /// Make a new client.
    pub async fn new(cache_dir: &Utf8Path) -> anyhow::Result<Self> {
        let lol_dir = cache_dir.join("lol");
        crate::util::try_create_dir(&lol_dir).await?;

        let profile_icons_dir = lol_dir.join("profile-icons");
        crate::util::try_create_dir(&profile_icons_dir).await?;

        let rank_icons_dir = lol_dir.join("rank-icons");
        crate::util::try_create_dir(&rank_icons_dir).await?;

        Ok(Self {
            client: ugg::Client::new(),
            icon_lock_map: AsyncMutexMap::new(),
            profile_icons_dir,
            rank_icon_lock_map: AsyncMutexMap::new(),
            rank_icons_dir,
            user_stats_renderer: Arc::new(tokio::sync::Mutex::new(UserStatsRenderer::new())),
        })
    }

    /// Search for a user.
    pub async fn search_users(&self, user: &str) -> anyhow::Result<Vec<ugg::model::User>> {
        info!("Searching for lol users with \"{user}\"");

        self.client
            .search_users(user)
            .await
            .with_context(|| format!("Failed to search for user \"{user}\""))
    }

    /// Get a player profile suggestion.
    pub async fn get_profile_suggestion(&self, user: &str) -> anyhow::Result<ugg::model::User> {
        info!("Searching for lol user with \"{user}\"");

        let mut users = self
            .client
            .search_users(user)
            .await
            .context("Failed to search for users")?;
        ensure!(!users.is_empty(), "Failed to find user from \"{user}\"");
        let user = users.swap_remove(0);
        Ok(user)
    }

    /// Update a player profile.
    pub async fn update_player_profile(
        &self,
        user_name: &str,
        tag_line: &str,
    ) -> anyhow::Result<()> {
        info!("Updating profile for lol user with \"{user_name}#{tag_line}\"");

        self.client
            .update_player_profile(user_name, tag_line)
            .await
            .with_context(|| {
                format!("Failed to update profile for user \"{user_name}#{tag_line}\"")
            })?;

        Ok(())
    }

    async fn download_to_path(&self, url: &str, path: &Utf8Path) -> anyhow::Result<()> {
        let temp_path = path.with_added_extension("temp");
        let mut file = tokio::fs::File::create(&temp_path).await?;
        let mut response = self
            .client
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?;
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        file.sync_all().await?;

        tokio::fs::rename(temp_path, &path).await?;

        anyhow::Ok(())
    }

    pub async fn get_icon_path(&self, icon_id: u32) -> anyhow::Result<Utf8PathBuf> {
        let lock = self.icon_lock_map.lock(icon_id).await;
        let path = self.profile_icons_dir.join(format!("{icon_id}.png"));

        if tokio::fs::try_exists(&path).await? {
            return Ok(path);
        }

        info!("Downloading lol profile icon {icon_id}");

        let url = format!(
            "https://static.bigbrain.gg/assets/lol/riot_static/16.22.1/img/profileicon/{icon_id}.png"
        );
        self.download_to_path(url.as_str(), &path).await?;

        drop(lock);

        Ok(path)
    }

    /// Get the path to the rank icon.
    ///
    /// Downloads if not already present.
    pub async fn get_rank_icon_path(&self, rank: &str) -> anyhow::Result<Utf8PathBuf> {
        let lock = self.rank_icon_lock_map.lock(rank.to_string()).await;
        let path = self.rank_icons_dir.join(format!("{rank}.png"));

        if tokio::fs::try_exists(&path).await? {
            return Ok(path);
        }

        info!("Downloading lol rank icon {rank}");

        let url = format!(
            "https://static.bigbrain.gg/assets/lol/ranks/s13/{rank}.png"
        );
        self.download_to_path(url.as_str(), &path).await?;

        drop(lock);

        Ok(path)
    }

    pub async fn render_user_stats(&self, user: ugg::model::User) -> anyhow::Result<Vec<u8>> {
        let icon_path = self.get_icon_path(user.icon_id).await?;
        let icon = tokio::fs::read(icon_path).await?;

        let rank_icon = match user.rank.as_ref() {
            Some(rank) => {
                let rank_icon_path = self.get_rank_icon_path(rank).await?;
                Some(tokio::fs::read(rank_icon_path).await?)
            }
            None => None,
        };

        let mut renderer = self.user_stats_renderer.clone().lock_owned().await;
        let png = tokio::task::spawn_blocking(move || {
            let start = Instant::now();
            let png = renderer.render(user, icon, rank_icon)?;
            let elapsed = start.elapsed();

            info!("Rendered lol user stats in {elapsed:?}");

            anyhow::Ok(png)
        })
        .await??;

        Ok(png)
    }
}

async fn autocomplete_user(ctx: PoiseContext<'_>, partial: &str) -> CreateAutocompleteResponse {
    if partial.is_empty() {
        return CreateAutocompleteResponse::new();
    }

    let ugg_client = &ctx.data().ugg_client;

    let users = match ugg_client.search_users(partial).await {
        Ok(users) => users,
        Err(error) => {
            warn!("{error:?}");
            return CreateAutocompleteResponse::new();
        }
    };
    let choices = users
        .iter()
        .map(|user| format!("{}#{}", user.riot_user_name, user.riot_tag_line).into())
        .collect();

    CreateAutocompleteResponse::new().set_choices(choices)
}

#[poise::command(
    slash_command,
    subcommands("stats", "user"),
    description_localized("en-US", "Get lol stats.")
)]
pub async fn lol(_ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    Ok(())
}
