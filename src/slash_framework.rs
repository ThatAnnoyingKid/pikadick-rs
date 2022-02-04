mod command;

pub use self::command::{
    SlashFrameworkCommand,
    SlashFrameworkCommandBuilder,
};
use anyhow::{
    ensure,
    Context as _,
};
use serenity::{
    model::{
        interactions::application_command::ApplicationCommand,
        prelude::*,
    },
    prelude::*,
};
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
};
use tracing::warn;

type OnProcessFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'static>>;

/// A framework
#[derive(Clone)]
pub struct SlashFramework {
    commands: Arc<HashMap<Box<str>, Arc<SlashFrameworkCommand>>>,
}

impl SlashFramework {
    /// Register the framework
    pub async fn register(
        &self,
        ctx: Context,
        test_guild_id: Option<GuildId>,
    ) -> anyhow::Result<()> {
        for (_name, framework_command) in self.commands.iter() {
            ApplicationCommand::create_global_application_command(&ctx.http, |command| {
                command
                    .name(&framework_command.name())
                    .description(&framework_command.description())
            })
            .await?;
        }

        if let Some(guild_id) = test_guild_id {
            GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
                for (_name, framework_command) in self.commands.iter() {
                    commands.create_application_command(|command| {
                        command
                            .name(&framework_command.name())
                            .description(&framework_command.description())
                    });
                }

                commands
            })
            .await?;
        }

        Ok(())
    }

    /// Process an interaction create event
    pub async fn process_interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let framework_command = match self.commands.get(command.data.name.as_str()) {
                Some(command) => command,
                None => {
                    // TODO: Unknown
                    return;
                }
            };

            if let Err(e) = framework_command
                .fire_on_process(ctx, command)
                .await
                .context("failed to process command")
            {
                // TODO: handle error with handler
                warn!("{:?}", e);
            }
        }
    }
}

impl std::fmt::Debug for SlashFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlashFramework")
            .field("commands", &self.commands)
            .finish()
    }
}

/// A FrameworkBuilder for slash commands.
pub struct SlashFrameworkBuilder {
    commands: HashMap<Box<str>, Arc<SlashFrameworkCommand>>,
}

impl SlashFrameworkBuilder {
    /// Make a new [`SlashFrameworkBuilder`].
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Add a command
    pub fn command(&mut self, command: SlashFrameworkCommand) -> anyhow::Result<&mut Self> {
        let command_name: Box<str> = command.name().into();
        let had_duplicate = self
            .commands
            .insert(command_name.clone(), Arc::new(command))
            .is_some();
        ensure!(!had_duplicate, "duplicate command for `{}`", command_name);
        Ok(self)
    }

    /// Build a framework
    pub fn build(&mut self) -> anyhow::Result<SlashFramework> {
        Ok(SlashFramework {
            commands: Arc::new(std::mem::take(&mut self.commands)),
        })
    }
}

impl std::fmt::Debug for SlashFrameworkBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlashFrameworkBuilder")
            .field("commands", &self.commands)
            .finish()
    }
}

impl Default for SlashFrameworkBuilder {
    fn default() -> Self {
        Self::new()
    }
}
