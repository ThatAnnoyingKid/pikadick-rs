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
        Ok(_old_value) => {
            let status_str = status_to_str(disable);

            // TODO: Tell user if the command is already disabled/enabled
            msg.channel_id
                .say(&ctx.http, format!("Command '{}' {}.", cmd_name, status_str))
                .await?;
        }
        Err(e) => {
            error!("failed to disable command '{}': {:?}", cmd_name, e);
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Failed to disable command '{}' ", cmd_name),
                )
                .await?;
        }
    }

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
    let client_data = data_lock.get::<ClientDataKey>().expect("client data");
    let data = client_data.enabled_check_data.clone();
    let db = client_data.db.clone();
    drop(data_lock);

    let res = {
        let mut res = "Commands:\n".to_string();

        let names = data.get_command_names().clone();

        for name in names.iter() {
            let state = if db.is_command_disabled(guild_id, name).await? {
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

fn status_to_str(status: bool) -> &'static str {
    if status {
        "disabled"
    } else {
        "enabled"
    }
}
