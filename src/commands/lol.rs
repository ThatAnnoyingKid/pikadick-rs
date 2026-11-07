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
use anyhow::Context;
use bewu_util::AsyncMutexMap;
use camino::{
    Utf8Path,
    Utf8PathBuf,
};
use std::{
    sync::Arc,
    time::Instant,
};
use tokio::io::AsyncWriteExt;
use tracing::info;

#[derive(Debug)]
pub struct UggClient {
    client: ugg::Client,
    icon_lock_map: AsyncMutexMap<u32>,
    profile_icons_dir: Utf8PathBuf,
    user_stats_renderer: Arc<tokio::sync::Mutex<UserStatsRenderer>>,
}

impl UggClient {
    /// Make a new client.
    pub async fn new(cache_dir: &Utf8Path) -> anyhow::Result<Self> {
        let lol_dir = cache_dir.join("lol");
        crate::util::try_create_dir(&lol_dir).await?;
        let profile_icons_dir = lol_dir.join("profile-icons");
        crate::util::try_create_dir(&profile_icons_dir).await?;

        Ok(Self {
            client: ugg::Client::new(),
            icon_lock_map: AsyncMutexMap::new(),
            profile_icons_dir,
            user_stats_renderer: Arc::new(tokio::sync::Mutex::new(UserStatsRenderer::new())),
        })
    }

    /// Search for a user.
    pub async fn search_users(&self, user: &str) -> anyhow::Result<Vec<ugg::model::User>> {
        info!("Searching for lol user with \"{user}\"");

        self.client
            .search_users(user)
            .await
            .with_context(|| format!("Failed to search for user \"{user}\""))
    }

    pub async fn get_icon_path(&self, icon_id: u32) -> anyhow::Result<Utf8PathBuf> {
        let lock = self.icon_lock_map.lock(icon_id).await;
        let icon_path = self.profile_icons_dir.join(format!("{icon_id}.png"));

        if tokio::fs::try_exists(&icon_path).await? {
            return Ok(icon_path);
        }

        info!("Downloading lol profile icon {icon_id}");

        let temp_path = icon_path.with_added_extension("temp");
        let url = format!(
            "https://static.bigbrain.gg/assets/lol/riot_static/16.22.1/img/profileicon/{icon_id}.png"
        );
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

        tokio::fs::rename(temp_path, &icon_path).await?;

        drop(lock);

        Ok(icon_path)
    }

    pub async fn render_user_stats(&self, user: ugg::model::User) -> anyhow::Result<Vec<u8>> {
        let icon_path = self.get_icon_path(user.icon_id).await?;
        let icon = tokio::fs::read(icon_path).await?;

        let mut renderer = self.user_stats_renderer.clone().lock_owned().await;
        let png = tokio::task::spawn_blocking(move || {
            let start = Instant::now();
            let png = renderer.render(user, icon)?;
            let elapsed = start.elapsed();

            info!("Rendered lol user stats in {elapsed:?}");

            anyhow::Ok(png)
        })
        .await??;

        Ok(png)
    }
}

#[poise::command(
    slash_command,
    subcommands("stats", "user"),
    description_localized("en-US", "Get lol stats.")
)]
pub async fn lol(_ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    Ok(())
}
