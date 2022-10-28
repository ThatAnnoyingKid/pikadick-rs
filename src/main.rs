#![deny(
    unused_qualifications,
    unused_qualifications,
    unused_import_braces,
    unused_lifetimes,
    unreachable_pub,
    trivial_numeric_casts,
    missing_debug_implementations,
    missing_copy_implementations,
    deprecated_in_future,
    meta_variable_misuse,
    non_ascii_idents,
    rust_2018_compatibility,
    rust_2018_idioms,
    future_incompatible,
    nonstandard_style,
    rustdoc::all,
    clippy::all,
    clippy::filter_map_next,
    clippy::ptr_as_ptr,
    clippy::cast_lossless,
    clippy::exit,
    clippy::filetype_is_file,
    clippy::macro_use_imports
)]
#![warn(
    clippy::borrow_as_ptr,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::cast_ptr_alignment,
    clippy::cloned_instead_of_copied,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::fn_params_excessive_bools,
    clippy::from_iter_instead_of_collect,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::items_after_statements,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_underscore_drop,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::manual_ok_or,
    clippy::match_bool,
    clippy::match_same_arms,
    clippy::mut_mut,
    clippy::mutex_atomic,
    clippy::mutex_integer,
    clippy::needless_for_each,
    clippy::nonstandard_macro_braces,
    clippy::path_buf_push_overwrite,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::redundant_else,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::semicolon_if_nothing_returned,
    clippy::suboptimal_flops,
    clippy::todo,
    clippy::transmute_ptr_to_ptr,
    clippy::trivially_copy_pass_by_ref,
    clippy::try_err,
    clippy::type_repetition_in_bounds,
    clippy::unicode_not_nfc,
    clippy::unnecessary_join,
    clippy::unnested_or_patterns,
    clippy::zero_sized_map_values
)]
#![allow(rustdoc::missing_doc_code_examples)]
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

//! # Pikadick

pub mod bot_context;
pub mod checks;
pub mod cli_options;
pub mod client_data;
pub mod commands;
pub mod config;
pub mod database;
pub mod logger;
pub mod setup;
pub mod util;

pub use crate::bot_context::BotContext;
use crate::{
    cli_options::CliOptions,
    client_data::ClientData,
    commands::*,
    config::{
        ActivityKind,
        Config,
    },
    database::{
        model::TikTokEmbedFlags,
        Database,
    },
    util::{
        is_reddit_host,
        is_tiktok_host,
        AsyncLockFile,
        TwilightLoadingReaction,
    },
};
use anyhow::{
    bail,
    ensure,
    Context as _,
};
use futures::StreamExt;
use once_cell::sync::Lazy;
use pikadick_slash_framework::ClientData as _;
use regex::Regex;
use serenity::{
    framework::standard::{
        macros::group,
        CommandResult,
        StandardFramework,
    },
    futures::future::BoxFuture,
    model::prelude::*,
    prelude::*,
    FutureExt,
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
    debug,
    error,
    info,
    warn,
};
use tracing_appender::non_blocking::WorkerGuard;
use twilight_cache_inmemory::{
    InMemoryCache,
    ResourceType,
};
use twilight_gateway::cluster::ClusterBuilder;
use twilight_model::gateway::{
    payload::outgoing::update_presence::UpdatePresencePayload,
    presence::{
        ActivityType,
        MinimalActivity,
        Status,
    },
    Intents,
};
use url::Url;

const TOKIO_RT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

/// Source: <https://urlregex.com/>
static URL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(include_str!("./url_regex.txt")).expect("invalid url regex"));

#[derive(Debug, Clone, Copy)]
pub struct ClientDataKey;

impl TypeMapKey for ClientDataKey {
    type Value = ClientData;
}

#[group]
#[commands(
    system,
    quizizz,
    fml,
    zalgo,
    shift,
    reddit_embed,
    invite,
    vaporwave,
    cmd,
    latency,
    tic_tac_toe,
    leave,
    stop
)]
struct General;

fn after_handler<'fut>(
    _ctx: &'fut Context,
    _msg: &'fut Message,
    command_name: &'fut str,
    command_result: CommandResult,
) -> BoxFuture<'fut, ()> {
    async move {
        if let Err(e) = command_result {
            error!("failed to process command '{}': {}", command_name, e);
        }
    }
    .boxed()
}

fn unrecognised_command_handler<'fut>(
    ctx: &'fut Context,
    msg: &'fut Message,
    command_name: &'fut str,
) -> BoxFuture<'fut, ()> {
    async move {
        error!("unrecognized command '{}'", command_name);

        let _ = msg
            .channel_id
            .say(
                &ctx.http,
                format!("Could not find command '{}'", command_name),
            )
            .await
            .is_ok();
    }
    .boxed()
}

/// Set up a serenity client
async fn setup_client(config: Arc<Config>) -> anyhow::Result<Client> {
    // Create second prefix that is uppercase so we are case-insensitive
    let config_prefix = config.prefix.clone();
    let uppercase_prefix = config_prefix.to_uppercase();

    // Build the standard framework
    info!("using prefix '{}'", &config_prefix);
    let framework = StandardFramework::new()
        .configure(|c| {
            c.prefixes(&[config_prefix, uppercase_prefix])
                .case_insensitivity(true)
        })
        .group(&GENERAL_GROUP)
        .bucket("system", |b| b.delay(30))
        .await
        .bucket("quizizz", |b| b.delay(10))
        .await
        .bucket("insta-dl", |b| b.delay(10))
        .await
        .bucket("ttt-board", |b| b.delay(1))
        .await
        .bucket("default", |b| b.delay(1))
        .await
        .after(after_handler)
        .unrecognised_command(unrecognised_command_handler);

    // Build the client
    let config_token = config.token.clone();
    let client = Client::builder(
        config_token.clone(),
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT,
    )
    .framework(framework)
    .register_songbird()
    .await
    .context("failed to create client")?;

    Ok(client)
}

/// Data from the setup function
struct SetupData {
    tokio_rt: tokio::runtime::Runtime,
    config: Arc<Config>,
    database: Database,
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

    let config = crate::setup::load_config(&cli_options.config)
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

    std::fs::create_dir_all(&config.log_file_dir()).context("failed to create log file dir")?;
    std::fs::create_dir_all(&config.cache_dir()).context("failed to create cache dir")?;

    eprintln!("opening database...");
    let database_path = config.data_dir.join("pikadick.sqlite");

    // Safety: This is called before any other sqlite functions.
    // TODO: Is there a good reason to not remake the db if it is missing?
    let database = unsafe {
        Database::blocking_new(&database_path, true) // missing_data_dir
            .context("failed to open database")?
    };

    // Everything past here is assumed to need tokio
    let _enter_guard = tokio_rt.handle().enter();

    eprintln!("setting up logger...");
    let worker_guard = crate::logger::setup(&config).context("failed to initialize logger")?;

    eprintln!();
    Ok(SetupData {
        tokio_rt,
        config,
        database,
        lock_file,
        worker_guard,
    })
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

    let setup_data = setup(cli_options)?;
    real_main(setup_data)?;
    Ok(())
}

/// The actual entry point
fn real_main(setup_data: SetupData) -> anyhow::Result<()> {
    // We spawn this is a seperate thread/task as the main thread does not have enough stack space
    let _enter_guard = setup_data.tokio_rt.enter();
    let ret = setup_data.tokio_rt.block_on(tokio::spawn(async_main(
        setup_data.config,
        setup_data.database,
    )));

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

/// The async entry
async fn async_main(config: Arc<Config>, database: Database) -> anyhow::Result<()> {
    // Setup slash framework
    let slash_framework = pikadick_slash_framework::FrameworkBuilder::new()
        .check(self::checks::enabled::create_slash_check)
        .help_command(create_slash_help_command()?)
        .command(self::commands::nekos::create_slash_command()?)
        .command(self::commands::ping::create_slash_command()?)
        .command(self::commands::r6stats::create_slash_command()?)
        .command(self::commands::r6tracker::create_slash_command()?)
        .command(self::commands::rule34::create_slash_command()?)
        .command(self::commands::tiktok_embed::create_slash_command()?)
        .command(self::commands::sauce_nao::create_slash_command()?)
        .command(self::commands::reddit::create_slash_command()?)
        .command(self::commands::iqdb::create_slash_command()?)
        .command(self::commands::xkcd::create_slash_command()?)
        .command(self::commands::urban::create_slash_command()?)
        .command(self::commands::deviantart::create_slash_command()?)
        .command(self::commands::insta_dl::create_slash_command()?)
        .command(self::commands::cache_stats::create_slash_command()?)
        .command(self::commands::uwuify::create_slash_command()?)
        .build()?;

    info!("starting shard cluster...");
    let mut cluster_builder = ClusterBuilder::new(
        config.token.clone(),
        Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT | Intents::DIRECT_MESSAGES,
    );

    // Set activity in cluster builder if in config
    if let (Some(status), Some(kind)) = (config.status_name(), config.status_type()) {
        let activity = match kind {
            ActivityKind::Listening => MinimalActivity {
                kind: ActivityType::Listening,
                name: status.to_string(),
                url: None,
            },
            ActivityKind::Streaming => MinimalActivity {
                kind: ActivityType::Streaming,
                name: status.to_string(),
                url: config.status_url().map(|s| s.to_string()),
            },
            ActivityKind::Playing => MinimalActivity {
                kind: ActivityType::Playing,
                name: status.to_string(),
                url: None,
            },
        };
        let payload =
            UpdatePresencePayload::new(vec![activity.into()], false, None, Status::Online)?;
        cluster_builder = cluster_builder.presence(payload);
    }
    let (cluster, mut events) = cluster_builder
        .build()
        .await
        .context("failed to create shard cluster")?;
    let cluster = Arc::new(cluster);
    let http = twilight_http::Client::new(config.token.clone());
    let cache = InMemoryCache::builder()
        .resource_types(
            ResourceType::MESSAGE
                | ResourceType::CHANNEL
                | ResourceType::VOICE_STATE
                | ResourceType::USER_CURRENT
                | ResourceType::ROLE
                | ResourceType::GUILD
                | ResourceType::MEMBER,
        )
        .build();

    // Set up ctrl+c handler
    {
        let cluster = cluster.clone();
        tokio::spawn(async move {
            if let Err(e) = tokio::signal::ctrl_c()
                .await
                .context("failed to register ctrl+c handler")
            {
                error!("{e:?}");
            }

            info!("got ctrl+c, shutting down...");
            cluster.down();
        });
    }

    cluster.up().await;
    info!("shard cluster is up");

    let bot_context = BotContext::new(http, config, slash_framework, database.clone())
        .await
        .context("failed to create bot context")?;
    while let Some((shard_id, event)) = events.next().await {
        cache.update(&event);
        tokio::spawn(handle_event(shard_id, event, bot_context.clone()));
    }
    info!("shard cluster is down");

    info!("closing encoder task...");
    if let Err(e) = bot_context
        .inner
        .encoder_task
        .shutdown()
        .await
        .context("failed to shutdown encoder task")
    {
        error!("{e:?}");
    }

    info!("closing database...");
    if let Err(e) = database.close().await.context("failed to close database") {
        error!("{e:?}");
    }

    Ok(())
}

async fn handle_event(shard_id: u64, event: twilight_gateway::Event, bot_context: BotContext) {
    debug!(shard_id = shard_id, "got event kind {:?}", event.kind());

    match event {
        twilight_gateway::Event::ShardConnecting(_) => {
            info!(shard_id = shard_id, "shard connecting");
        }
        twilight_gateway::Event::ShardConnected(_) => {
            info!(shard_id = shard_id, "shard connected");
        }
        twilight_gateway::Event::ShardDisconnected(_) => {
            info!(shard_id = shard_id, "shard disconnected");
        }
        twilight_gateway::Event::ShardIdentifying(_) => {
            info!(shard_id = shard_id, "shard identifying");
        }
        twilight_gateway::Event::ShardReconnecting(_) => {
            info!(shard_id = shard_id, "shard reconnecting");
        }
        twilight_gateway::Event::ShardResuming(_) => {
            info!(shard_id = shard_id, "shard resuming");
        }
        twilight_gateway::Event::Ready(ready) => {
            info!(shard_id = shard_id, "shard ready");

            info!("logged in as '{}'", ready.user.name);

            // Attempt to register slash commands
            let interaction_client = bot_context.interaction_client();

            // TODO: Consider shutting down the bot. It might be possible to use old data though.
            if let Err(e) = bot_context
                .inner
                .slash_framework
                .register(
                    interaction_client,
                    bot_context.inner.config.test_guild.map(|id| id.into()),
                )
                .await
                .context("failed to register slash commands")
            {
                error!("{e:?}");
            }
            info!("registered slash commands");
        }
        twilight_gateway::Event::Resumed => {
            info!(shard_id = shard_id, "shard resumed");
        }
        twilight_gateway::Event::MessageCreate(message_create) => {
            // Process URL embeds

            // Only embed guild links
            let guild_id = match message_create.guild_id {
                Some(id) => id,
                None => {
                    return;
                }
            };

            // No Bots
            if message_create.author.bot {
                return;
            }

            // Get enabled data for embeds
            let reddit_embed_is_enabled_for_guild = bot_context
                .inner
                .database
                .get_reddit_embed_enabled(guild_id.into_nonzero().into())
                .await
                .with_context(|| format!("failed to get reddit-embed server data for '{guild_id}'"))
                .unwrap_or_else(|e| {
                    error!("{e:?}");
                    false
                });
            let tiktok_embed_flags = bot_context
                .inner
                .database
                .get_tiktok_embed_flags(guild_id.into_nonzero().into())
                .await
                .with_context(|| format!("failed to get tiktok-embed server data for '{guild_id}'"))
                .unwrap_or_else(|e| {
                    error!("{e:?}");
                    TikTokEmbedFlags::empty()
                });

            // Extract urls
            // NOTE: Regex doesn't HAVE to be perfect.
            // Ideally, it just needs to be aggressive since parsing it into a url will weed out invalids.
            //
            // We collect into a `Vec` as the regex iterator is not Sync and cannot be held across await points.
            let urls: Vec<Url> = URL_REGEX
                .find_iter(&message_create.content)
                .filter_map(|url_match| Url::parse(url_match.as_str()).ok())
                .collect();

            // Check to see if it we will even try to embed
            let will_try_embedding = urls.iter().any(|url| {
                let url_host = match url.host() {
                    Some(host) => host,
                    None => return false,
                };

                let is_reddit_url = is_reddit_host(&url_host);
                let is_tiktok_url = is_tiktok_host(&url_host);

                (is_reddit_url && reddit_embed_is_enabled_for_guild)
                    || (is_tiktok_url && tiktok_embed_flags.contains(TikTokEmbedFlags::ENABLED))
            });

            // Return if we won't try embedding
            if !will_try_embedding {
                return;
            }

            // TODO: Port loadingreaction to twilight
            let mut loading_reaction = Some(TwilightLoadingReaction::new(
                bot_context.clone(),
                message_create.0.channel_id,
                message_create.0.id,
            ));

            // Embed for each url
            // NOTE: we short circuit on failure since sending a msg to a channel and failing is most likely a permissions problem.
            for url in urls.iter() {
                let url_host = match url.host() {
                    Some(host) => host,
                    None => continue,
                };

                let is_reddit_url = is_reddit_host(&url_host);
                let is_tiktok_url = is_tiktok_host(&url_host);

                if is_reddit_url {
                    // Don't process if it isn't enabled
                    if reddit_embed_is_enabled_for_guild {
                        if let Err(e) = bot_context
                            .inner
                            .reddit_embed_data
                            .try_embed_url(
                                &bot_context,
                                &message_create,
                                url,
                                &mut loading_reaction,
                            )
                            .await
                            .context("failed to generate reddit embed")
                        {
                            error!("{e:?}");
                        }
                    }
                } else if is_tiktok_url {
                    // Don't process if it isn't enabled
                    if tiktok_embed_flags.contains(TikTokEmbedFlags::ENABLED) {
                        if let Err(e) = bot_context
                            .inner
                            .tiktok_data
                            .try_embed_url(
                                &bot_context,
                                &message_create,
                                url,
                                &mut loading_reaction,
                                tiktok_embed_flags.contains(TikTokEmbedFlags::DELETE_LINK),
                            )
                            .await
                            .context("failed to generate tiktok embed")
                        {
                            error!("{e:?}");
                        }
                    }
                }
            }

            // Trim caches
            bot_context.inner.reddit_embed_data.cache.trim();
            bot_context.inner.reddit_embed_data.video_data_cache.trim();
            bot_context.inner.tiktok_data.post_page_cache.trim();
        }
        twilight_gateway::Event::InteractionCreate(interaction_create) => {
            bot_context
                .inner
                .slash_framework
                .process_interaction_create(bot_context.clone(), interaction_create)
                .await;
        }
        _ => {}
    }
}
