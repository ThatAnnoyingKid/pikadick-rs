#![deny(
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
    clippy::all
)]
#![warn(variant_size_differences, let_underscore_drop)]
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
#![allow(deprecated)]

//! # Pikadick

pub mod checks;
pub mod cli_options;
pub mod client_data;
pub mod commands;
pub mod config;
pub mod database;
pub mod logger;
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
        model::TikTokEmbedFlags,
        Database,
    },
    util::LoadingReaction,
};
use anyhow::{
    bail,
    ensure,
    Context as _,
};
use pikadick_util::AsyncLockFile;
use serenity::{
    framework::standard::{
        buckets::BucketBuilder,
        help_commands,
        macros::{
            group,
            help,
        },
        Args,
        CommandGroup,
        CommandResult,
        Configuration as StandardFrameworkConfiguration,
        DispatchError,
        HelpOptions,
        Reason,
        StandardFramework,
    },
    futures::future::BoxFuture,
    gateway::{
        ActivityData,
        ShardManager,
    },
    model::{
        application::Interaction,
        prelude::*,
    },
    prelude::*,
    FutureExt,
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
use tokio::runtime::Builder as RuntimeBuilder;
use tracing::{
    error,
    info,
    warn,
};
use tracing_appender::non_blocking::WorkerGuard;
use url::Url;

const TOKIO_RT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        let data_lock = ctx.data.read().await;
        let client_data = data_lock
            .get::<ClientDataKey>()
            .expect("missing client data");
        let slash_framework = data_lock
            .get::<SlashFrameworkKey>()
            .expect("missing slash framework")
            .clone();
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

        // TODO: Consider shutting down the bot. It might be possible to use old data though.
        if let Err(error) = slash_framework
            .register(ctx.clone(), config.test_guild)
            .await
            .context("failed to register slash commands")
        {
            error!("{error:?}");
        }

        info!("registered slash commands");
    }

    async fn resume(&self, _ctx: Context, _resumed: ResumedEvent) {
        warn!("resumed connection");
    }

    #[tracing::instrument(skip(self, ctx, msg), fields(author = %msg.author.id, guild = ?msg.guild_id, content = %msg.content))]
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
                            if let Err(error) = reddit_embed_data
                                .try_embed_url(&ctx, &msg, url, &mut loading_reaction)
                                .await
                                .context("failed to generate reddit embed")
                            {
                                error!("{error:?}");
                            }
                        }
                    }
                    Some(url::Host::Domain("vm.tiktok.com" | "tiktok.com" | "www.tiktok.com")) => {
                        if tiktok_embed_flags.contains(TikTokEmbedFlags::ENABLED) {
                            if let Err(error) = tiktok_data
                                .try_embed_url(
                                    &ctx,
                                    &msg,
                                    url,
                                    &mut loading_reaction,
                                    tiktok_embed_flags.contains(TikTokEmbedFlags::DELETE_LINK),
                                )
                                .await
                                .context("failed to generate tiktok embed")
                            {
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
            tiktok_data.post_page_cache.trim();
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let data_lock = ctx.data.read().await;
        let framework = data_lock
            .get::<SlashFrameworkKey>()
            .expect("missing slash framework")
            .clone();
        drop(data_lock);

        framework.process_interaction_create(ctx, interaction).await;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClientDataKey;

impl TypeMapKey for ClientDataKey {
    type Value = ClientData;
}

#[derive(Debug, Clone, Copy)]
pub struct SlashFrameworkKey;

impl TypeMapKey for SlashFrameworkKey {
    type Value = pikadick_slash_framework::Framework;
}

#[help]
async fn help(
    ctx: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    match help_commands::with_embeds(ctx, msg, args, help_options, groups, owners)
        .await
        .context("failed to send help")
    {
        Ok(_) => {}
        Err(error) => {
            error!("{error:?}");
        }
    }
    Ok(())
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
    uwuify,
    cache_stats,
    insta_dl,
    deviantart,
    urban,
    xkcd,
    tic_tac_toe,
    iqdb,
    reddit,
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
            warn!("{error}");
            // The default "kill everything" handler is probably still installed, so this isn't a problem?
        }
    };
}

#[tracing::instrument(skip(_ctx, msg), fields(author = %msg.author.id, guild = ?msg.guild_id, content = %msg.content))]
fn before_handler<'fut>(
    _ctx: &'fut Context,
    msg: &'fut Message,
    cmd_name: &'fut str,
) -> BoxFuture<'fut, bool> {
    info!("allowing command to process");
    async move { true }.boxed()
}

fn after_handler<'fut>(
    _ctx: &'fut Context,
    _msg: &'fut Message,
    command_name: &'fut str,
    command_result: CommandResult,
) -> BoxFuture<'fut, ()> {
    async move {
        if let Err(error) = command_result {
            error!("failed to process command \"{command_name}\": {error}");
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
        error!("unrecognized command \"{command_name}\"");

        let _ = msg
            .channel_id
            .say(
                &ctx.http,
                format!("Could not find command \"{command_name}\""),
            )
            .await
            .is_ok();
    }
    .boxed()
}

fn process_dispatch_error<'fut>(
    ctx: &'fut Context,
    msg: &'fut Message,
    error: DispatchError,
    cmd_name: &'fut str,
) -> BoxFuture<'fut, ()> {
    process_dispatch_error_future(ctx, msg, error, cmd_name).boxed()
}

async fn process_dispatch_error_future<'fut>(
    ctx: &'fut Context,
    msg: &'fut Message,
    error: DispatchError,
    _cmd_name: &'fut str,
) {
    match error {
        DispatchError::Ratelimited(duration) => {
            let seconds = duration.as_secs();
            let _ = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!("Wait {seconds} seconds to use that command again"),
                )
                .await
                .is_ok();
        }
        DispatchError::NotEnoughArguments { min, given } => {
            let _ = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!(
                        "Expected at least {min} argument(s) for this command, but only got {given}",
                    ),
                )
                .await
                .is_ok();
        }
        DispatchError::TooManyArguments { max, given } => {
            let response_str = format!("Expected no more than {max} argument(s) for this command, but got {given}. Try using quotation marks if your argument has spaces.");
            let _ = msg.channel_id.say(&ctx.http, response_str).await.is_ok();
        }
        DispatchError::CheckFailed(check_name, reason) => match reason {
            Reason::User(user_reason_str) => {
                let _ = msg.channel_id.say(&ctx.http, user_reason_str).await.is_ok();
            }
            _ => {
                let _ = msg
                    .channel_id
                    .say(
                        &ctx.http,
                        format!("\"{check_name}\" check failed: {reason:#?}"),
                    )
                    .await
                    .is_ok();
            }
        },
        error => {
            let _ = msg
                .channel_id
                .say(&ctx.http, format!("Unhandled Dispatch Error: {error:?}"))
                .await
                .is_ok();
        }
    };
}

/// Set up a serenity client
async fn setup_client(config: Arc<Config>) -> anyhow::Result<Client> {
    // Setup slash framework
    let slash_framework = pikadick_slash_framework::FrameworkBuilder::new()
        .check(self::checks::enabled::create_slash_check)
        .help_command(create_slash_help_command()?)
        .command(nekos::create_slash_command()?)
        .command(ping::create_slash_command()?)
        .command(r6stats::create_slash_command()?)
        .command(r6tracker::create_slash_command()?)
        .command(rule34::create_slash_command()?)
        .command(tiktok_embed::create_slash_command()?)
        .command(chat::create_slash_command()?)
        .command(yodaspeak::create_slash_command()?)
        .build()?;

    // Create second prefix that is uppercase so we are case-insensitive
    let config_prefix = config.prefix.clone();
    let uppercase_prefix = config_prefix.to_uppercase();

    // Build the standard framework
    info!("using prefix \"{config_prefix}\"");
    let framework_config = StandardFrameworkConfiguration::new()
        .prefixes([config_prefix, uppercase_prefix])
        .case_insensitivity(true);
    let framework = StandardFramework::new();
    framework.configure(framework_config);
    let framework = framework
        .help(&HELP)
        .group(&GENERAL_GROUP)
        .bucket("r6stats", BucketBuilder::new_channel().delay(7))
        .await
        .bucket("r6tracker", BucketBuilder::new_channel().delay(7))
        .await
        .bucket("system", BucketBuilder::new_channel().delay(30))
        .await
        .bucket("quizizz", BucketBuilder::new_channel().delay(10))
        .await
        .bucket("insta-dl", BucketBuilder::new_channel().delay(10))
        .await
        .bucket("ttt-board", BucketBuilder::new_channel().delay(1))
        .await
        .bucket("default", BucketBuilder::new_channel().delay(1))
        .await
        .before(before_handler)
        .after(after_handler)
        .unrecognised_command(unrecognised_command_handler)
        .on_dispatch_error(process_dispatch_error);

    // Build the client
    let config_token = config.token.clone();
    let client = Client::builder(
        config_token,
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT,
    )
    .event_handler(Handler)
    .application_id(ApplicationId::new(config.application_id))
    .framework(framework)
    .register_songbird()
    .await
    .context("failed to create client")?;

    {
        client
            .data
            .write()
            .await
            .insert::<SlashFrameworkKey>(slash_framework);
    }

    // TODO: Spawn a task for this earlier?
    // Spawn the ctrl-c handler
    tokio::spawn(handle_ctrl_c(client.shard_manager.clone()));

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

    // TODO: Init db
    eprintln!("opening database...");
    let database_path = config.data_dir.join("pikadick.sqlite");

    // Safety: This is called before any other sqlite functions.
    // TODO: Is there a good reason to not remake the db if it is missing?
    let database = unsafe {
        Database::blocking_new(database_path, true) // missing_data_dir
            .context("failed to open database")?
    };

    // Everything past here is assumed to need tokio
    let _enter_guard = tokio_rt.handle().enter();

    eprintln!("setting up logger...");
    let worker_guard = logger::setup(&config).context("failed to initialize logger")?;

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
    // TODO: See if it is possible to start serenity without a network
    info!("setting up client...");
    let mut client = setup_client(config.clone())
        .await
        .context("failed to set up client")?;

    let client_data = ClientData::init(client.shard_manager.clone(), config, database.clone())
        .await
        .context("client data initialization failed")?;

    // Add all post-init client data changes here
    {
        client_data.enabled_check_data.add_groups(&[&GENERAL_GROUP]);
    }

    {
        let mut data = client.data.write().await;
        data.insert::<ClientDataKey>(client_data);
    }

    info!("logging in...");
    client.start().await.context("failed to run client")?;
    let client_data = {
        let mut data = client.data.write().await;
        data.remove::<ClientDataKey>().expect("missing client data")
    };
    drop(client);

    info!("running shutdown routine for client data");
    client_data.shutdown().await;
    drop(client_data);

    info!("closing database...");
    database.close().await.context("failed to close database")?;

    Ok(())
}
