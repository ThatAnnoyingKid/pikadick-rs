use crate::{
    checks::{
        ADMIN_CHECK,
        ENABLED_CHECK,
    },
    ClientDataKey,
};
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use std::fmt::Write;
use tracing::error;

// Broken in help:
// #[required_permissions("ADMINISTRATOR")]

#[command]
#[description("Disable a command")]
#[usage("<enable/disable> <cmd>")]
#[example("disable ping")]
#[min_args(2)]
#[max_args(2)]
#[sub_commands(list)]
#[checks(Admin, Enabled)]
#[bucket("default")]
pub async fn cmd(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            msg.channel_id
                .say(&ctx.http, "You cannot use this in a DM.")
                .await?;
            return Ok(());
        }
    };

    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let data = client_data.enabled_check_data.clone();
    let db = client_data.db.clone();
    drop(data_lock);

    let disable = match args.current().expect("invalid arg") {
        "enable" => false,
        "disable" => true,
        _ => {
            msg.channel_id
                .say(&ctx.http, "Invalid Arg. Choose \"enable\" or \"disable\".")
                .await?;
            return Ok(());
        }
    };

    args.advance();

    let cmd_name = args.current().expect("missing cmd name");

    let is_valid_command = {
        let names = data.get_command_names();
        names.iter().any(|name| name == cmd_name)
    };

    if !is_valid_command {
        msg.channel_id
            .say(
                &ctx.http,
                "Invalid Command. Use `cmd list` to list valid commands.",
            )
            .await?;
        return Ok(());
    }

    match db.set_disabled_command(guild_id, cmd_name, disable).await {
        Ok(()) => {}
        Err(e) => {
            error!("Failed to disable command: {}", e);
        }
    }

    let status_str = if disable { "disabled" } else { "enabled" };

    msg.channel_id
        .say(&ctx.http, format!("Command '{}' {}.", cmd_name, status_str))
        .await?;

    Ok(())
}

#[command]
#[description("List commands that can be enabled/disabled")]
pub async fn list(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            msg.channel_id
                .say(&ctx.http, "You cannot use this in a DM.")
                .await?;
            return Ok(());
        }
    };

    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let data = client_data.enabled_check_data.clone();
    let db = client_data.db.clone();
    drop(data_lock);

    let disabled_commands = match db.get_disabled_commands(guild_id).await {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to get disabled commands: {}", e);
            return Ok(());
        }
    };

    let res = {
        let mut res = "Commands:\n".to_string();

        let names = data.get_command_names();

        for name in names.iter() {
            let state = if disabled_commands.contains(name) {
                "DISABLED"
            } else {
                "ENABLED"
            };
            writeln!(res, "{}: **{}**", name, state)?;
        }

        res
    };

    msg.channel_id.say(&ctx.http, res).await?;
    Ok(())
}
