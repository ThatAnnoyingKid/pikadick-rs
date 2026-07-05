#![deny(
    unused_import_braces,
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
    missing_debug_implementations
)]
// TODO: Document everything properly
// clippy::default_trait_access
// clippy::use_self
// clippy::undocumented_unsafe_blocks
// clippy::allow_attributes_without_reason
// clippy::as_underscore
// clippy::cast_possible_truncation
// clippy::cast_possible_wrap
// clippy::cast_sign_loss
// clippy::fn_to_numeric_cast_any
// clippy::redundant_closure_for_method_calls
// clippy::too_many_lines

// TODO: Switch to poise
#![expect(deprecated)]

//! # Pikadick

pub mod checks;
pub mod cli_options;
pub mod client_data;
pub mod commands;
pub mod config;
pub mod database;
pub mod logger;
mod poise_data;
pub mod setup;
pub mod util;

use crate::{
    cli_options::CliOptions,
    client_data::ClientData,
    commands::*,
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
use poise::structs::FrameworkError;
use serenity::{
    FutureExt,
    framework::standard::macros::group,
    gateway::{
        ActivityData,
        ShardManager,
    },
    model::prelude::*,
    prelude::*,
};
use songbird::SerenityInit;
use std::{
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};
use tokio::runtime::Builder as RuntimeBuilder;
use tracing::{
    error,
    info,
    warn,
};
use tracing_appender::non_blocking::WorkerGuard;
use url::Url;

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

struct Handler;

type PoiseData = Arc<crate::poise_data::PoiseData>;
type PoiseError = Box<dyn std::error::Error + Send + Sync>;
type PoiseContext<'a> = poise::Context<'a, PoiseData, PoiseError>;

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        let data_lock = ctx.data.read().await;
        let client_data = data_lock
            .get::<ClientDataKey>()
            .expect("missing client data");
        let config = client_data.config.clone();
        drop(data_lock);

        if let (Some(status), Some(kind)) = (config.status_name(), config.status_type()) {
            match kind {
                ActivityKind::Listening => {
                    ctx.set_activity(Some(ActivityData::listening(status)));
                }
                ActivityKind::Streaming => {
                    let result: Result<_, anyhow::Error> = async {
                        let activity = ActivityData::streaming(
                            status,
                            config.status_url().context("failed to get status url")?,
                        )?;

                        ctx.set_activity(Some(activity));

                        Ok(())
                    }
                    .await;

                    if let Err(error) = result.context("failed to set activity") {
                        error!("{error:?}");
                    }
                }
                ActivityKind::Playing => {
                    ctx.set_activity(Some(ActivityData::playing(status)));
                }
            }
        }

        info!("logged in as \"{}\"", ready.user.name);
    }

    async fn resume(&self, _ctx: Context, _resumed: ResumedEvent) {
        warn!("resumed connection");
    }

    #[tracing::instrument(skip(self, ctx, msg), fields(author = %msg.author.id, guild = ?msg.guild_id, content = %msg.content))]
    #[expect(clippy::collapsible_match)]
    async fn message(&self, ctx: Context, msg: Message) {
        let data_lock = ctx.data.read().await;
        let client_data = data_lock
            .get::<ClientDataKey>()
            .expect("missing client data");
        let reddit_embed_data = client_data.reddit_embed_data.clone();
        let tiktok_data = client_data.tiktok_data.clone();
        let db = client_data.db.clone();
        drop(data_lock);

        // Process URL Embeds
        {
            // Only embed guild links
            let guild_id = match msg.guild_id {
                Some(id) => id,
                None => {
                    return;
                }
            };

            // No Bots
            if msg.author.bot {
                return;
            }

            // Get enabled data for embeds
            let reddit_embed_is_enabled_for_guild = db
                .get_reddit_embed_enabled(guild_id)
                .await
                .with_context(|| format!("failed to get reddit-embed server data for {guild_id}"))
                .unwrap_or_else(|error| {
                    error!("{error:?}");
                    false
                });
            let tiktok_embed_flags = db
                .get_tiktok_embed_flags(guild_id)
                .await
                .with_context(|| format!("failed to get tiktok-embed server data for {guild_id}"))
                .unwrap_or_else(|error| {
                    error!("{error:?}");
                    TikTokEmbedFlags::empty()
                });

            // Extract urls.
            // We collect into a `Vec` as the regex iterator is not Sync and cannot be held across await points.
            let urls: Vec<Url> = util::extract_urls(&msg.content).collect();

            // Check to see if it we will even try to embed
            let will_try_embedding = urls.iter().any(|url| {
                let url_host = match url.host() {
                    Some(host) => host,
                    None => return false,
                };

                let reddit_url =
                    matches!(url_host, url::Host::Domain("www.reddit.com" | "reddit.com"));

                let tiktok_url = matches!(
                    url_host,
                    url::Host::Domain("vm.tiktok.com" | "tiktok.com" | "www.tiktok.com")
                );

                (reddit_url && reddit_embed_is_enabled_for_guild)
                    || (tiktok_url && tiktok_embed_flags.contains(TikTokEmbedFlags::ENABLED))
            });

            // Return if we won't try embedding
            if !will_try_embedding {
                return;
            }

            let mut loading_reaction = Some(LoadingReaction::new(ctx.http.clone(), &msg));

            // Embed for each url
            // NOTE: we short circuit on failure since sending a msg to a channel and failing is most likely a permissions problem,
            // especially since serenity retries each req once
            for url in urls.iter() {
                match url.host() {
                    Some(url::Host::Domain("www.reddit.com" | "reddit.com")) => {
                        // Don't process if it isn't enabled
                        if reddit_embed_is_enabled_for_guild {
                            let result = reddit_embed_data
                                .try_embed_url(&ctx, &msg, url, &mut loading_reaction)
                                .await
                                .context("failed to generate reddit embed");
                            if let Err(error) = result {
                                error!("{error:?}");
                            }
                        }
                    }
                    Some(url::Host::Domain("vm.tiktok.com" | "tiktok.com" | "www.tiktok.com")) => {
                        if tiktok_embed_flags.contains(TikTokEmbedFlags::ENABLED) {
                            let result = tiktok_data
                                .try_embed_url(
                                    &ctx,
                                    &msg,
                                    url,
                                    &mut loading_reaction,
                                    tiktok_embed_flags.contains(TikTokEmbedFlags::DELETE_LINK),
                                )
                                .await
                                .context("failed to generate tiktok embed");
                            if let Err(error) = result {
                                error!("{error:?}");
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Trim caches
            reddit_embed_data.cache.trim();
            reddit_embed_data.video_data_cache.trim();
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClientDataKey;

impl TypeMapKey for ClientDataKey {
    type Value = ClientData;
}

#[group]
#[commands(
    system,
    reddit_embed,
    cmd,
    latency,
    insta_dl,
    deviantart,
    tic_tac_toe,
    iqdb,
    leave,
    stop,
    sauce_nao
)]
struct General;

async fn handle_ctrl_c(shard_manager: Arc<ShardManager>) {
    match tokio::signal::ctrl_c()
        .await
        .context("failed to set ctrl-c handler")
    {
        Ok(_) => {
            info!("shutting down...");
            info!("stopping client...");
            shard_manager.shutdown_all().await;
        }
        Err(error) => {
            error!("{error}");
            shard_manager.shutdown_all().await;
        }
    };
}

/// Set up a serenity client
async fn setup_client(config: Arc<Config>, database: Database) -> anyhow::Result<Client> {
    let poise_data = Arc::new(PoiseDataInner::new(config.clone(), database).await?);

    let framework_options = poise::FrameworkOptions {
        commands: vec![
            self::commands::help(),
            self::commands::nekos(),
            self::commands::ping(),
            self::commands::tiktok_embed(),
            self::commands::quizizz(),
            self::commands::r6tracker(),
            self::commands::reddit(),
            self::commands::rule34(),
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
                            .reply("This command can only be used in nsfw channels")
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
                                "Wait {seconds:2} seconds to use that command again"
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
        ..Default::default()
    };

    let framework = {
        let config = config.clone();
        poise::Framework::builder()
            .options(framework_options)
            .setup(move |ctx, _ready, framework| {
                Box::pin(async move {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    if let Some(test_guild_id) = config.test_guild {
                        poise::builtins::register_in_guild(
                            ctx,
                            &framework.options().commands,
                            test_guild_id,
                        )
                        .await?;
                    }
                    info!("registered commands");

                    Ok::<PoiseData, PoiseError>(poise_data)
                })
            })
            .build()
    };

    // Build the client
    let client = Client::builder(
        config.token.clone(),
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT,
    )
    .event_handler(Handler)
    .application_id(ApplicationId::new(config.application_id))
    .framework(framework)
    .register_songbird()
    .await
    .context("failed to create client")?;

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
    eprintln!("starting tokio runtime...");
    let tokio_rt = RuntimeBuilder::new_multi_thread()
        .enable_all()
        .thread_name("pikadick-tokio-worker")
        .build()
        .context("failed to start tokio runtime")?;

    let config = setup::load_config(&cli_options.config)
        .map(Arc::new)
        .context("failed to load config")?;

    eprintln!("opening data directory...");
    let data_dir_metadata = match std::fs::metadata(&config.data_dir) {
        Ok(metadata) => Some(metadata),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(e).context("failed to get metadata for the data dir");
        }
    };

    let _missing_data_dir = data_dir_metadata.is_none();
    match data_dir_metadata.as_ref() {
        Some(metadata) => {
            if metadata.is_dir() {
                eprintln!("data directory already exists.");
            } else if metadata.is_file() {
                bail!("failed to create or open data directory, the path is a file");
            }
        }
        None => {
            eprintln!("data directory does not exist. creating...");
            std::fs::create_dir_all(&config.data_dir).context("failed to create data directory")?;
        }
    }

    eprintln!("creating lockfile...");
    let lock_file_path = config.data_dir.join("pikadick.lock");
    let lock_file = AsyncLockFile::blocking_open(lock_file_path.as_std_path())
        .context("failed to open lockfile")?;
    let lock_file_locked = lock_file
        .try_lock_with_pid_blocking()
        .context("failed to try to lock the lockfile")?;
    ensure!(lock_file_locked, "another process has locked the lockfile");

    std::fs::create_dir_all(config.log_file_dir()).context("failed to create log file dir")?;
    std::fs::create_dir_all(config.cache_dir()).context("failed to create cache dir")?;

    // Everything past here is assumed to need tokio
    let _enter_guard = tokio_rt.handle().enter();

    eprintln!("setting up logger...");
    let worker_guard = logger::setup(&config).context("failed to initialize logger")?;

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
    let database_path = config.data_dir.join("pikadick.sqlite");
    let database = Database::new(database_path)
        .await
        .context("Failed to open database")?;

    // TODO: See if it is possible to start serenity without a network
    info!("Setting up client...");
    let mut client = setup_client(config.clone(), database.clone())
        .await
        .context("Failed to set up client")?;

    let client_data = ClientData::init(client.shard_manager.clone(), config, database.clone())
        .await
        .context("Client data initialization failed")?;

    // Add all post-init client data changes here
    {
        client_data.enabled_check_data.add_groups(&[&GENERAL_GROUP]);
    }

    {
        let mut data = client.data.write().await;
        data.insert::<ClientDataKey>(client_data);
    }

    info!("Logging in...");
    client.start().await.context("Failed to run client")?;
    let client_data = {
        let mut data = client.data.write().await;
        data.remove::<ClientDataKey>().expect("Missing client data")
    };
    drop(client);

    info!("Running shutdown routine for client data...");
    client_data.shutdown().await;
    drop(client_data);

    info!("Closing database...");
    database.close().await.context("failed to close database")?;

    Ok(())
}

/// The actual entry point
fn real_main(setup_data: SetupData) -> anyhow::Result<()> {
    // We spawn this is a seperate thread/task as the main thread does not have enough stack space
    let _enter_guard = setup_data.tokio_rt.enter();
    let ret = setup_data
        .tokio_rt
        .block_on(tokio::spawn(async_main(setup_data.config)));

    let shutdown_start = Instant::now();
    info!(
        "shutting down tokio runtime (shutdown timeout is {:?})...",
        TOKIO_RT_SHUTDOWN_TIMEOUT
    );
    setup_data
        .tokio_rt
        .shutdown_timeout(TOKIO_RT_SHUTDOWN_TIMEOUT);
    info!("shutdown tokio runtime in {:?}", shutdown_start.elapsed());

    info!("unlocking lockfile...");
    setup_data
        .lock_file
        .blocking_unlock()
        .context("failed to unlock lockfile")?;

    info!("successful shutdown");

    // Logging no longer reliable past this point
    drop(setup_data.worker_guard);

    ret?
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
            .context("failed to install sqlite log handler")?;
    }

    let setup_data = setup(cli_options)?;
    real_main(setup_data)?;

    Ok(())
}
