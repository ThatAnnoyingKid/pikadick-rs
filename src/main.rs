#![deny(
    unused_qualifications,
    clippy::all,
    unused_qualifications,
    unused_import_braces,
    // unused_lifetimes, // TODO: Enable. Seems buggy?
    unreachable_pub,
    trivial_numeric_casts,
    rustdoc::all,
    missing_debug_implementations,
    missing_copy_implementations,
    deprecated_in_future,
    meta_variable_misuse,
    non_ascii_idents,
    rust_2018_compatibility,
    rust_2018_idioms,
    future_incompatible,
    nonstandard_style
)]
#![allow(missing_doc_code_examples)] // TODO: Document everything properly

//! # Pikadick

pub mod checks;
pub mod client_data;
pub mod commands;
pub mod config;
pub mod database;
pub mod logger;
pub mod util;

use crate::{
    client_data::ClientData,
    commands::*,
    config::{
        ActivityKind,
        Config,
        Severity,
    },
    database::Database,
};
use anyhow::Context as _;
use serenity::{
    client::bridge::gateway::ShardManager,
    framework::standard::{
        help_commands,
        macros::{
            group,
            help,
        },
        Args,
        CommandGroup,
        CommandResult,
        DispatchError,
        HelpOptions,
        Reason,
        StandardFramework,
    },
    futures::future::BoxFuture,
    model::prelude::*,
    prelude::*,
    FutureExt,
};
use std::{
    collections::HashSet,
    path::Path,
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

const TOKIO_RT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

struct Handler;

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
                    ctx.set_activity(Activity::listening(status)).await;
                }
                ActivityKind::Streaming => {
                    ctx.set_activity(Activity::streaming(status, config.status_url().unwrap()))
                        .await;
                }
                ActivityKind::Playing => {
                    ctx.set_activity(Activity::playing(status)).await;
                }
            }
        }

        info!("Logged in as '{}'", ready.user.name);
    }

    async fn resume(&self, _ctx: Context, resumed: ResumedEvent) {
        warn!("Resumed connection. Trace: {:?}", resumed.trace);
    }

    #[tracing::instrument(skip(self, ctx, msg), fields(author = %msg.author.id, guild = ?msg.guild_id, content = %msg.content))]
    async fn message(&self, ctx: Context, msg: Message) {
        let data_lock = ctx.data.read().await;
        let client_data = data_lock
            .get::<ClientDataKey>()
            .expect("missing client data");
        let reddit_embed_data = client_data.reddit_embed_data.clone();
        drop(data_lock);

        if let Err(e) = reddit_embed_data.process_msg(&ctx, &msg).await {
            error!("Failed to generate reddit embed: {}", e);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClientDataKey;

impl TypeMapKey for ClientDataKey {
    type Value = ClientData;
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
    let _ = help_commands::with_embeds(ctx, msg, args, help_options, groups, owners)
        .await
        .is_some();
    Ok(())
}

#[group]
#[commands(
    ping,
    nekos,
    r6stats,
    r6tracker,
    rule34,
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
    iqdb
)]
struct General;

async fn handle_ctrl_c(shard_manager: Arc<Mutex<ShardManager>>) {
    match tokio::signal::ctrl_c().await {
        Ok(_) => {
            info!("Shutting down...");
            info!("Stopping Client...");
            shard_manager.lock().await.shutdown_all().await;
        }
        Err(e) => {
            warn!("Failed to set Ctrl-C handler: {}", e);
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
    info!("Allowing command to process");
    async move { true }.boxed()
}

fn after_handler<'fut>(
    _ctx: &'fut Context,
    _msg: &'fut Message,
    command_name: &'fut str,
    command_result: CommandResult,
) -> BoxFuture<'fut, ()> {
    async move {
        if let Err(e) = command_result {
            error!("Failed to process command '{}': {}", command_name, e);
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
        error!("Unrecognized command '{}'", command_name);

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

fn process_dispatch_error<'fut>(
    ctx: &'fut Context,
    msg: &'fut Message,
    error: DispatchError,
) -> BoxFuture<'fut, ()> {
    process_dispatch_error_future(ctx, msg, error).boxed()
}

async fn process_dispatch_error_future<'fut>(
    ctx: &'fut Context,
    msg: &'fut Message,
    error: DispatchError,
) {
    match error {
        DispatchError::Ratelimited(s) => {
            let _ = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!("Wait {} seconds to use that command again", s.as_secs()),
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
                        "Expected at least {} argument(s) for this command, but only got {}",
                        min, given
                    ),
                )
                .await
                .is_ok();
        }
        DispatchError::TooManyArguments { max, given } => {
            let response_str = format!("Expected no more than {} argument(s) for this command, but got {}. Try using quotation marks if your argument has spaces.",
                max, given
            );
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
                        format!("{} check failed: {:#?}", check_name, reason),
                    )
                    .await
                    .is_ok();
            }
        },
        e => {
            let _ = msg
                .channel_id
                .say(&ctx.http, format!("Unhandled Dispatch Error: {:?}", e))
                .await
                .is_ok();
        }
    };
}

/// Load a config.
///
/// This prints to the stderr directly.
/// It is intended to be called BEFORE the loggers are set up.
fn load_config() -> anyhow::Result<Config> {
    let config_path: &Path = "./config.toml".as_ref();

    eprintln!("Loading `{}`...", config_path.display());
    let mut config = Config::load_from_path(config_path)
        .with_context(|| format!("failed to load `{}`", config_path.display()))?;

    eprintln!("Validating config...");
    let errors = config.validate();
    let mut error_count = 0;
    for e in errors {
        match e.severity() {
            Severity::Warn => {
                eprintln!("Validation Warning: {}", e.error());
            }
            Severity::Error => {
                eprintln!("Validation Error: {}", e.error());
                error_count += 1;
            }
        }
    }

    if error_count != 0 {
        anyhow::bail!("Validation failed with {} errors.", error_count);
    }

    Ok(config)
}

/// Pre-main setup
fn setup() -> anyhow::Result<(tokio::runtime::Runtime, Config, bool, WorkerGuard)> {
    eprintln!("Starting tokio runtime...");
    let tokio_rt = RuntimeBuilder::new_multi_thread()
        .enable_all()
        .thread_name("pikadick-tokio-worker")
        .build()
        .context("failed to start Tokio Runtime")?;

    let config = load_config().context("failed to load config")?;

    eprintln!("Opening data directory...");
    if config.data_dir.is_file() {
        anyhow::bail!("failed to create or open data directory, the path is a file");
    }

    let missing_data_dir = !config.data_dir.exists();
    if missing_data_dir {
        eprintln!("Data directory does not exist. Creating...");
        std::fs::create_dir_all(&config.data_dir).context("failed to create data directory")?;
    } else if config.data_dir.is_dir() {
        eprintln!("Data directory already exists.");
    }

    std::fs::create_dir_all(&config.log_file_dir()).context("failed to create log file dir")?;

    eprintln!("Setting up logger...");
    let guard = tokio_rt
        .block_on(async { crate::logger::setup(&config) })
        .context("failed to initialize logger")?;

    eprintln!();
    Ok((tokio_rt, config, missing_data_dir, guard))
}

/// The main entry.
///
/// Calls `real_main` and prints the error, exiting with 1 of needed.
/// This allows more things to drop correctly.
/// This also calls setup operations like loading config and setting up the tokio runtime,
/// logging errors to the stderr instead of the loggers, which are not initialized yet.
fn main() {
    let (tokio_rt, config, missing_data_dir, worker_guard) = match setup() {
        Ok(data) => data,
        Err(e) => {
            eprintln!("{:?}", e);
            drop(e);

            std::process::exit(1);
        }
    };

    let exit_code = match real_main(tokio_rt, config, missing_data_dir, worker_guard) {
        Ok(()) => 0,
        Err(e) => {
            error!("{:?}", e);
            1
        }
    };

    std::process::exit(exit_code);
}

/// The actual entry point
fn real_main(
    tokio_rt: tokio::runtime::Runtime,
    config: Config,
    missing_data_dir: bool,
    _worker_guard: WorkerGuard,
) -> anyhow::Result<()> {
    tokio_rt.block_on(async_main(config, missing_data_dir));

    let shutdown_start = Instant::now();
    info!(
        "Shutting down tokio runtime (shutdown timeout is {:?})...",
        TOKIO_RT_SHUTDOWN_TIMEOUT
    );
    tokio_rt.shutdown_timeout(TOKIO_RT_SHUTDOWN_TIMEOUT);
    info!("Shutdown tokio runtime in {:?}", shutdown_start.elapsed());

    info!("Successful Shutdown");
    Ok(())
}

/// The async entry
async fn async_main(config: Config, missing_data_dir: bool) {
    info!("Opening database...");
    let db_path = config.data_dir.join("pikadick.sqlite");
    let db = match Database::new(&db_path, missing_data_dir)
        .await
        .context("failed to open database")
    {
        Ok(db) => db,
        Err(e) => {
            error!("{:?}", e);
            return;
        }
    };

    let uppercase_prefix = config.prefix.to_uppercase();
    let framework = StandardFramework::new()
        .configure(|c| {
            c.prefixes(&[&config.prefix, &uppercase_prefix])
                .case_insensitivity(true)
        })
        .help(&HELP)
        .group(&GENERAL_GROUP)
        .bucket("nekos", |b| b.delay(1))
        .await
        .bucket("r6stats", |b| b.delay(7))
        .await
        .bucket("r6tracker", |b| b.delay(7))
        .await
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
        .before(before_handler)
        .after(after_handler)
        .unrecognised_command(unrecognised_command_handler)
        .on_dispatch_error(process_dispatch_error);

    info!("Using prefix '{}'", &config.prefix);

    let mut client = match Client::builder(&config.token)
        .event_handler(Handler)
        .framework(framework)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create client: {}", e);
            return;
        }
    };

    tokio::spawn(handle_ctrl_c(client.shard_manager.clone()));

    let client_data = match ClientData::init(client.shard_manager.clone(), config, db).await {
        Ok(c) => {
            // Add all post-init client data changes here
            c.enabled_check_data.add_groups(&[&GENERAL_GROUP]);
            c
        }
        Err(e) => {
            error!("Client Data Initialization failed: {}", e);
            return;
        }
    };

    {
        let mut data = client.data.write().await;
        data.insert::<ClientDataKey>(client_data);
    }

    info!("Logging in...");
    if let Err(why) = client.start().await {
        error!("Error while running client: {}", why);
    }

    drop(client);
}
