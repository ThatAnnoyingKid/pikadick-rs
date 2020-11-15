use crate::ClientDataKey;
use parking_lot::Mutex;
use serenity::{
    client::Context,
    framework::standard::{
        macros::check,
        Args,
        CheckResult,
        CommandGroup,
        CommandOptions,
    },
    model::prelude::*,
};
use slog::error;
use std::sync::Arc;

type MutexGuard<'a, T> = parking_lot::lock_api::MutexGuard<'a, parking_lot::RawMutex, T>;

#[derive(Debug, Default, Clone)]
pub struct EnabledCheckData {
    pub groups: Vec<&'static CommandGroup>,
    pub command_name_cache: Arc<Mutex<Option<Vec<String>>>>,
}

impl EnabledCheckData {
    /// Returns a mutex guard to the command cache. Guaranteed to be Some.
    pub fn get_command_names(&self) -> MutexGuard<'_, Option<Vec<String>>> {
        let mut cache = self.command_name_cache.lock();
        cache.get_or_insert_with(|| {
            let mut commands = Vec::new();
            for group in self.groups.iter() {
                // let base = group.name;

                for cmd in group.options.commands {
                    if cmd
                        .options
                        .checks
                        .iter()
                        .any(|check| check.name == "Enabled")
                    {
                        if let Some(cmd_name) = cmd.options.names.first() {
                            commands.push(cmd_name.to_string()); // format!("{}::{}", base, cmd_name)
                        }
                    }
                }
            }

            commands
        });

        cache
    }
}

#[check]
#[name("Enabled")]
pub async fn enabled_check(
    ctx: &Context,
    msg: &Message,
    _args: &mut Args,
    opts: &CommandOptions,
) -> CheckResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            // Let's not care about dms for now.
            // They'll probably need special handling anyways.
            // This will also probably only be useful in Group DMs,
            // which I don't think bots can participate in anyways.
            return true.into();
        }
    };

    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let db = client_data.db.clone();
    let logger = client_data.logger.clone();
    drop(data_lock);

    let disabled_commands = match db.get_disabled_commands(guild_id).await {
        Ok(data) => data,
        Err(e) => {
            error!(logger, "Failed to read disabled commands: {}", e);

            // DB failure, return false to be safe.
            return false.into();
        }
    };

    let cmd_name = opts.names.first().expect("1 Valid Command Name");

    if disabled_commands.contains(*cmd_name) {
        return CheckResult::new_user("Command Disabled.");
    }

    true.into()
}
