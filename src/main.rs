#![recursion_limit = "256"]
#![deny(
    unused_lifetimes,
    trivial_numeric_casts,
    deprecated_in_future,
    meta_variable_misuse,
    non_ascii_idents,
    rust_2018_compatibility,
    rust_2018_idioms,
    future_incompatible,
    nonstandard_style
)]
#![warn(
    variant_size_differences,
    let_underscore_drop,
    missing_debug_implementations,
    clippy::undocumented_unsafe_blocks
)]
// TODO: Document everything properly
// clippy::default_trait_access
// clippy::use_self
// clippy::allow_attributes_without_reason
// clippy::as_underscore
// clippy::cast_possible_truncation
// clippy::cast_possible_wrap
// clippy::cast_sign_loss
// clippy::fn_to_numeric_cast_any
// clippy::redundant_closure_for_method_calls
// clippy::too_many_lines

//! # Pikadick

mod cli_options;
mod commands;
mod config;
mod database;
mod logger;
mod poise_data;
mod setup;
mod util;

use crate::{
    cli_options::CliOptions,
    config::{
        ActivityKind,
        Config,
    },
    database::{
        Database,
        model::TikTokEmbedFlags,
    },
    poise_data::PoiseData as PoiseDataInner,
    util::LoadingReaction,
};
use anyhow::{
    Context as _,
    bail,
    ensure,
};
use mimalloc::MiMalloc;
use pikadick_util::AsyncLockFile;
use poise::{
    dispatch::FrameworkContext,
    serenity_prelude::FullEvent,
    structs::FrameworkError,
};
use serenity::{
    FutureExt,
    gateway::{
        ActivityData,
        ShardManager,
    },
    model::prelude::*,
    prelude::*,
};
use songbird::SerenityInit;
use std::{
    collections::HashSet,
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};
use tracing::{
    error,
    info,
    warn,
};
use tracing_appender::non_blocking::WorkerGuard;
use url::Url;

type PoiseData = Arc<crate::poise_data::PoiseData>;
type PoiseError = anyhow::Error;
type PoiseContext<'a> = poise::Context<'a, PoiseData, PoiseError>;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

const TOKIO_RT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

fn rusqlite_log_handler(error_code: i32, message: &str) {
    use nd_async_rusqlite::rusqlite::ffi::{
        Error,
        SQLITE_NOTICE,
        SQLITE_WARNING,
    };

    let error = Error::new(error_code);

    match error_code & 0xff {
        // Couldn't figure out how to dedupe this..
        SQLITE_NOTICE => tracing::info!(
            target: "sqlite",
            code = %error,
            "{message}",
        ),
        SQLITE_WARNING => tracing::warn!(
            target: "sqlite",
            code = %error,
            "{message}",
        ),
        _ => tracing::error!(
            target: "sqlite",
            code = %error,
            "{message}",
        ),
    }
}

async fn handle_ctrl_c(shard_manager: Arc<ShardManager>) {
    match tokio::signal::ctrl_c()
        .await
        .context("Failed to set ctrl-c handler")
    {
        Ok(_) => {
            info!("Shutting down...");
            info!("Stopping client...");
            shard_manager.shutdown_all().await;
        }
        Err(error) => {
            error!("{error}");
            shard_manager.shutdown_all().await;
        }
    };
}

async fn process_urls_in_message(
    ctx: &FrameworkContext<'_, PoiseData, PoiseError>,
    msg: &Message,
) -> anyhow::Result<()> {
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    enum UrlType {
        TikTok,
    }

    let tiktok_data = &ctx.user_data.tiktok_data;

    // Only embed guild links
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            return Ok(());
        }
    };

    // No Bots
    if msg.author.bot {
        return Ok(());
    }

    let mut embed_url_types = HashSet::new();
    let tiktok_embed_flags = ctx
        .user_data
        .database
        .get_tiktok_embed_flags(guild_id)
        .await
        .with_context(|| format!("Failed to get tiktok-embed server data for {guild_id}"))
        .unwrap_or_else(|error| {
            error!("{error:?}");
            TikTokEmbedFlags::empty()
        });
    if tiktok_embed_flags.contains(TikTokEmbedFlags::ENABLED) {
        embed_url_types.insert(UrlType::TikTok);
    }

    // If there are no url types to embed, exit.
    if embed_url_types.is_empty() {
        return Ok(());
    }

    // Extract urls.
    let urls: Vec<(UrlType, Url)> = crate::util::extract_urls(&msg.content)
        .filter_map(|url| {
            let url_host = match url.host() {
                Some(host) => host,
                None => return None,
            };

            let url_type = match url_host {
                url::Host::Domain("vm.tiktok.com" | "tiktok.com" | "www.tiktok.com") => {
                    UrlType::TikTok
                }
                _ => return None,
            };

            Some((url_type, url))
        })
        .filter(|(url_type, _url)| embed_url_types.contains(url_type))
        .collect();

    if urls.is_empty() {
        return Ok(());
    }

    let mut loading_reaction = Some(LoadingReaction::new(ctx.serenity_context.http.clone(), msg));
    for (url_type, url) in urls.iter() {
        match url_type {
            UrlType::TikTok => {
                tiktok_data
                    .try_embed_url(
                        ctx.serenity_context,
                        msg,
                        url,
                        &mut loading_reaction,
                        tiktok_embed_flags.contains(TikTokEmbedFlags::DELETE_LINK),
                    )
                    .await
                    .context("Failed to generate tiktok embed")?;
            }
        }
    }

    Ok(())
}

async fn set_status(ctx: &FrameworkContext<'_, PoiseData, PoiseError>) -> anyhow::Result<()> {
    let config = &ctx.user_data.config;
    let ctx = ctx.serenity_context;

    if let (Some(status), Some(kind)) = (config.status_name(), config.status_type()) {
        match kind {
            ActivityKind::Listening => {
                ctx.set_activity(Some(ActivityData::listening(status)));
            }
            ActivityKind::Streaming => {
                let result = async {
                    let activity = ActivityData::streaming(
                        status,
                        config.status_url().context("Failed to get status url")?,
                    )?;

                    ctx.set_activity(Some(activity));

                    anyhow::Ok(())
                }
                .await;

                if let Err(error) = result.context("Failed to set activity") {
                    error!("{error:?}");
                }
            }
            ActivityKind::Playing => {
                ctx.set_activity(Some(ActivityData::playing(status)));
            }
        }
    }

    Ok(())
}

/// Set up a serenity client
async fn setup_client(poise_data: PoiseData) -> anyhow::Result<Client> {
    let config = poise_data.config.clone();

    let framework_options = poise::FrameworkOptions {
        commands: vec![
            self::commands::deviantart(),
            self::commands::iqdb(),
            self::commands::latency(),
            self::commands::leave(),
            self::commands::nekos(),
            self::commands::ping(),
            self::commands::quizizz(),
            self::commands::r6tracker(),
            self::commands::rule34(),
            self::commands::sauce_nao(),
            self::commands::stop(),
            self::commands::system(),
            self::commands::tic_tac_toe(),
            self::commands::tiktok_embed(),
            self::commands::urban(),
            self::commands::uwuify(),
            self::commands::vaporwave(),
            self::commands::xkcd(),
            self::commands::yodaspeak(),
            self::commands::zalgo(),
        ],
        on_error: |error| {
            (async move {
                match error {
                    FrameworkError::CommandCheckFailed { ctx, .. } => {
                        let result = ctx.reply("Command check failed").await;
                        if let Err(error) = result {
                            error!("{error}");
                        }
                    }
                    FrameworkError::NsfwOnly { ctx, .. } => {
                        let result = ctx
                            .reply("This command can only be used in nsfw channels.")
                            .await;
                        if let Err(error) = result {
                            error!("{error}");
                        }
                    }
                    FrameworkError::Command { ctx, error, .. } => {
                        warn!("{error:?}");

                        let result = ctx.reply(format!("{error}")).await;
                        if let Err(error) = result {
                            error!("{error}");
                        }
                    }
                    FrameworkError::CooldownHit {
                        ctx,
                        remaining_cooldown,
                        ..
                    } => {
                        let seconds = remaining_cooldown.as_secs_f64();
                        let result = ctx
                            .reply(format!(
                                "Wait {seconds:2} seconds to use that command again."
                            ))
                            .await;
                        if let Err(error) = result {
                            error!("{error}");
                        }
                    }
                    _ => {
                        error!("{error}");

                        if let Some(ctx) = error.ctx() {
                            let result = ctx.reply(format!("{error}")).await;
                            if let Err(error) = result {
                                error!("{error}");
                            }
                        }
                    }
                }
            })
            .boxed()
        },
        event_handler: |context, event| {
            async move {
                match event {
                    FullEvent::Message { new_message } => {
                        process_urls_in_message(&context, new_message).await?;
                    }
                    FullEvent::Ready { data_about_bot } => {
                        set_status(&context).await?;
                        info!("Logged in as \"{}\"", data_about_bot.user.name);
                    }
                    FullEvent::Resume { .. } => {
                        warn!("Resumed connection");
                    }
                    _ => {}
                }
                anyhow::Ok(())
            }
            .boxed()
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .options(framework_options)
        .setup(move |ctx, _ready, framework| {
            async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                if let Some(test_guild_id) = poise_data.config.test_guild {
                    poise::builtins::register_in_guild(
                        ctx,
                        &framework.options().commands,
                        test_guild_id,
                    )
                    .await?;
                }
                info!("Registered commands");

                anyhow::Ok(poise_data)
            }
            .boxed()
        })
        .build();

    // Build the client
    let client = Client::builder(
        config.token.clone(),
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT,
    )
    .application_id(ApplicationId::new(config.application_id))
    .framework(framework)
    .register_songbird()
    .await
    .context("Failed to create client")?;

    // TODO: Spawn a task for this earlier?
    // Spawn the ctrl-c handler
    tokio::spawn(handle_ctrl_c(client.shard_manager.clone()));

    Ok(client)
}

/// Data from the setup function
struct SetupData {
    tokio_rt: tokio::runtime::Runtime,
    config: Arc<Config>,
    lock_file: AsyncLockFile,
    worker_guard: WorkerGuard,
}

/// Pre-main setup
fn setup(cli_options: CliOptions) -> anyhow::Result<SetupData> {
    eprintln!("Starting tokio runtime...");
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to start tokio runtime")?;

    let config = setup::load_config(&cli_options.config)
        .map(Arc::new)
        .context("Failed to load config")?;

    eprintln!("Opening data directory...");
    let data_dir_metadata = match std::fs::metadata(&config.data_dir) {
        Ok(metadata) => Some(metadata),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(e).context("Failed to get metadata for the data dir");
        }
    };

    let _missing_data_dir = data_dir_metadata.is_none();
    match data_dir_metadata.as_ref() {
        Some(metadata) => {
            if metadata.is_dir() {
                eprintln!("Data directory already exists.");
            } else if metadata.is_file() {
                bail!("Failed to create or open data directory, the path is a file");
            }
        }
        None => {
            eprintln!("Data directory does not exist. creating...");
            std::fs::create_dir_all(&config.data_dir).context("Failed to create data directory")?;
        }
    }

    eprintln!("Creating lockfile...");
    let lock_file_path = config.data_dir.join("pikadick.lock");
    let lock_file = AsyncLockFile::blocking_open(lock_file_path.as_std_path())
        .context("Failed to open lockfile")?;
    let lock_file_locked = lock_file
        .try_lock_with_pid_blocking()
        .context("Failed to try to lock the lockfile")?;
    ensure!(lock_file_locked, "Another process has locked the lockfile");

    std::fs::create_dir_all(config.log_file_dir()).context("Failed to create log file dir")?;
    std::fs::create_dir_all(config.cache_dir()).context("Failed to create cache dir")?;

    // Everything past here is assumed to need tokio
    let _enter_guard = tokio_rt.handle().enter();

    eprintln!("Setting up logger...");
    let worker_guard = logger::setup(&config).context("Failed to initialize logger")?;

    eprintln!();
    Ok(SetupData {
        tokio_rt,
        config,
        lock_file,
        worker_guard,
    })
}

/// The async entry
async fn async_main(config: Arc<Config>) -> anyhow::Result<()> {
    const DATABASE_NAME: &str = "pikadick.sqlite";

    let database_path = config.data_dir.join(DATABASE_NAME);
    let database = Database::new(database_path)
        .await
        .context("Failed to open database")?;

    // TODO: See if it is possible to start serenity without a network
    info!("Setting up client...");
    let poise_data = Arc::new(PoiseDataInner::new(config.clone(), database.clone()).await?);
    let mut client = setup_client(poise_data.clone())
        .await
        .context("Failed to set up client")?;

    info!("Logging in...");
    client.start().await.context("Failed to run client")?;
    drop(client);

    info!("Running shutdown routine for client data...");
    poise_data.shutdown().await;
    drop(poise_data);

    info!("Closing database...");
    database.close().await.context("Failed to close database")?;

    Ok(())
}

/// The actual entry point
fn real_main(setup_data: SetupData) -> anyhow::Result<()> {
    // We spawn this is a seperate thread/task as the main thread does not have enough stack space
    let _enter_guard = setup_data.tokio_rt.enter();
    let result = setup_data
        .tokio_rt
        .block_on(tokio::spawn(async_main(setup_data.config)));

    let shutdown_start = Instant::now();
    info!("Shutting down tokio runtime (shutdown timeout is {TOKIO_RT_SHUTDOWN_TIMEOUT:?})...");
    setup_data
        .tokio_rt
        .shutdown_timeout(TOKIO_RT_SHUTDOWN_TIMEOUT);
    let elapsed = shutdown_start.elapsed();
    info!("Shutdown tokio runtime in {elapsed:?}");

    info!("Unlocking lockfile...");
    setup_data
        .lock_file
        .blocking_unlock()
        .context("Failed to unlock lockfile")?;

    info!("Successful shutdown");

    // Logging no longer reliable past this point
    drop(setup_data.worker_guard);

    result?
}

/// The main entry.
///
/// Sets up the program and calls `real_main`.
/// This allows more things to drop correctly.
/// This also calls setup operations like loading config and setting up the tokio runtime,
/// logging errors to the stderr instead of the loggers, which are not initialized yet.
fn main() -> anyhow::Result<()> {
    // This line MUST run first.
    // It is needed to exit early if the options are invalid,
    // and this will NOT run destructors if it does so.
    let cli_options = argh::from_env();

    // Safety:
    // 1. SQLite has not been called yet.
    // 2. The logging callback does not invoke SQLite.
    // 3. The logging callback is threadsafe.
    unsafe {
        nd_async_rusqlite::rusqlite::trace::config_log(Some(rusqlite_log_handler))
            .context("Failed to install sqlite log handler")?;
    }

    let setup_data = setup(cli_options)?;
    real_main(setup_data)?;

    Ok(())
}
