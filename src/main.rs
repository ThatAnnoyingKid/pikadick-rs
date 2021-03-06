#![deny(
    unused_qualifications,
    clippy::all,
    unused_qualifications,
    unused_import_braces,
    // unused_lifetimes, // TODO: Enable. Seems buggy?
    unreachable_pub,
    trivial_numeric_casts,
    rustdoc,
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
        ConfigError,
        Severity,
    },
    database::Database,
};
use log::{
    error,
    info,
    warn,
};
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
use sqlx::sqlite::SqlitePool;
use std::{
    collections::HashSet,
    sync::Arc,
};
use tokio::runtime::Builder as RuntimeBuilder;

struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        let data_lock = ctx.data.read().await;
        let client_data = data_lock.get::<ClientDataKey>().unwrap();
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

    async fn resume(&self, _ctx: Context, _resumed: ResumedEvent) {
        warn!("Resumed connection");
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let data_lock = ctx.data.read().await;
        let client_data = data_lock.get::<ClientDataKey>().unwrap();
        let reddit_embed_data = client_data.reddit_embed_data.clone();
        drop(data_lock);

        match reddit_embed_data.process_msg(&ctx, &msg).await {
            Ok(()) => {}
            Err(e) => {
                error!("Failed to generate reddit embed: {:?}", e);
            }
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
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, &help_options, groups, owners).await;
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
    deviantart
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
        DispatchError::CheckFailed(check_name, reason) => match check_name {
            "Admin" => {
                let _ = msg
                    .channel_id
                    .say(
                        &ctx.http,
                        "You need to be admin in order to use this command",
                    )
                    .await
                    .is_ok();
            }
            _ => match reason {
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

fn main() {
    let log_file_writer = match crate::logger::setup() {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to init logger: {}", e);
            return;
        }
    };

    info!("Loading config.toml...");
    let mut config = match Config::load_from_path("./config.toml".as_ref()) {
        Ok(c) => c,
        Err(e) => {
            match e {
                ConfigError::DoesNotExist(p) => {
                    error!("Failed to load {}. The file does not exist.", p.display());
                }
                ConfigError::IsNotFile(p) => {
                    error!("Failed to load {}. The path is not a file.", p.display());
                }
                _ => {
                    error!("Failed to load ./config.toml: {}", e);
                }
            }
            return;
        }
    };

    info!("Validating config.toml...");
    let errors = config.validate();
    let mut error_count = 0;
    for e in errors {
        match e.severity() {
            Severity::Warn => {
                warn!("Validation Warning: {}", e.error());
            }
            Severity::Error => {
                error!("Validation Error: {}", e.error());
                error_count += 1;
            }
        }
    }

    if error_count != 0 {
        error!("Validation failed with {} errors.", error_count);
        return;
    }

    info!("Opening data directory...");
    let data_dir = config.data_dir();
    let db_path = data_dir.join("pikadick.sqlite");

    if data_dir.is_file() {
        error!("Failed to create or open data directory, the path is a file.");
        return;
    }

    if !data_dir.exists() {
        info!("Data directory does not exist. Creating...");
        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            error!("Failed to create data directory: {}", e);
            return;
        };
    } else if data_dir.is_dir() {
        info!("Data directory already exists.");
    }

    info!("Initalizing File Logger...");
    let log_file = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(data_dir.join("log.txt"))
    {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to initalize file logger: {}", e);
            return;
        }
    };

    if let Err(e) = log_file_writer.init(log_file) {
        error!("Failed to initalize file logger: {}", e);
        return;
    }

    drop(log_file_writer);

    info!("Starting Tokio Runtime...");
    let tokio_rt = match RuntimeBuilder::new_multi_thread()
        .enable_all()
        .thread_name("pikadick-tokio-worker")
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            error!("Failed to start Tokio Runtime: {}", e);
            return;
        }
    };

    tokio_rt.block_on(async {
        // TODO: Add similar to sql
        // let mut db_options = rocksdb::Options::default();
        // db_options.create_if_missing(true);
        info!("Opening database...");
        let db_url = format!("sqlite:{}", db_path.display());
        let db = match SqlitePool::connect(&db_url).await {
            Ok(db) => match Database::new(db).await {
                Ok(db) => db,
                Err(e) => {
                    error!("Failed to initalize database: {}", e);
                    return;
                }
            },
            Err(e) => {
                error!("Failed to open database: {}", e);
                return;
            }
        };

        let uppercase_prefix = config.prefix().to_uppercase();
        let framework = StandardFramework::new()
            .configure(|c| {
                c.prefixes(&[config.prefix(), &uppercase_prefix])
                    .case_insensitivity(true)
            })
            .help(&HELP)
            .group(&GENERAL_GROUP)
            // .bucket("nekos", |b| b.delay(1)) // TODO: Consider better ratelimit strategy
            .bucket("r6stats", |b| b.delay(7))
            .await
            .bucket("r6tracker", |b| b.delay(7))
            .await
            .bucket("system", |b| b.delay(30))
            .await
            .bucket("quizizz", |b| b.delay(10))
            .await
            .after(after_handler)
            .unrecognised_command(unrecognised_command_handler)
            .on_dispatch_error(process_dispatch_error);

        info!("Using prefix '{}'", config.prefix());

        let mut client = match Client::builder(config.token())
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

        let client_data = match ClientData::init(client.shard_manager.clone(), config, db).await {
            Ok(mut c) => {
                // Add all post-init client data changes here
                c.enabled_check_data.groups.push(&GENERAL_GROUP);
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

        tokio::spawn(handle_ctrl_c(client.shard_manager.clone()));

        if let Err(why) = client.start().await {
            error!("Error while running client: {}", why);
        }

        drop(client);
    });

    info!("Stopping Tokio Runtime...");
    // TODO: Add a timeout to always shut down properly / Can i report when this fails?
    // tokio_rt.shutdown_timeout(TOKIO_RT_SHUTDOWN_DURATION);
    // Avoid using shutdown_timeout. Blocked on: https://github.com/tokio-rs/tokio/issues/2314
    drop(tokio_rt);

    info!("Successful Shutdown");
}
