use anyhow::{
    ensure,
    Context as _,
};
use serenity::{
    model::{
        interactions::application_command::{
            ApplicationCommand,
            ApplicationCommandInteraction,
        },
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
pub struct Framework {
    commands: Arc<HashMap<Box<str>, Arc<FrameworkCommand>>>,
}

impl Framework {
    /// Register the framework
    pub async fn register(&self, ctx: Context) -> anyhow::Result<()> {
        for (_name, framework_command) in self.commands.iter() {
            ApplicationCommand::create_global_application_command(&ctx.http, |command| {
                command
                    .name(&framework_command.name)
                    .description(&framework_command.description)
            })
            .await?;
        }

        let guild_id = GuildId(282036235776819201);
        GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            for (_name, framework_command) in self.commands.iter() {
                commands.create_application_command(|command| {
                    command
                        .name(&framework_command.name)
                        .description(&framework_command.description)
                });
            }

            commands
        })
        .await?;

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

            if let Err(e) = (framework_command.on_process)(ctx, command)
                .await
                .context("failed to process command")
            {
                // TODO: handle error with handler
                warn!("{:?}", e);
            }
        }
    }
}

impl std::fmt::Debug for Framework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Framework")
            .field("commands", &self.commands)
            .finish()
    }
}

/// A FrameworkBuilder for slash commands.
pub struct FrameworkBuilder {
    commands: HashMap<Box<str>, Arc<FrameworkCommand>>,
}

impl FrameworkBuilder {
    /// Make a new [`FrameworkBuilder`].
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Add a command
    pub fn command(&mut self, command: FrameworkCommand) -> anyhow::Result<&mut Self> {
        let command_name = command.name.clone();
        let had_duplicate = self
            .commands
            .insert(command_name.clone(), Arc::new(command))
            .is_some();
        ensure!(!had_duplicate, "duplicate command for `{}`", command_name);
        Ok(self)
    }

    /// Build a framework
    pub fn build(&mut self) -> anyhow::Result<Framework> {
        Ok(Framework {
            commands: Arc::new(std::mem::take(&mut self.commands)),
        })
    }
}

impl std::fmt::Debug for FrameworkBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameworkBuilder")
            .field("commands", &self.commands)
            .finish()
    }
}

impl Default for FrameworkBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A framework command
pub struct FrameworkCommand {
    /// The name of the command
    name: Box<str>,

    /// Description
    description: Box<str>,

    on_process: Box<
        dyn Fn(Context, ApplicationCommandInteraction) -> OnProcessFuture + Send + Sync + 'static,
    >,
}

impl std::fmt::Debug for FrameworkCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameworkCommand")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("on_process", &"<func>")
            .finish()
    }
}

/// A builder for a [`FrameworkCommand`].
pub struct FrameworkCommandBuilder<'a, 'b> {
    name: Option<&'a str>,
    description: Option<&'b str>,

    on_process: Option<
        Box<
            dyn Fn(Context, ApplicationCommandInteraction) -> OnProcessFuture
                + Send
                + Sync
                + 'static,
        >,
    >,
}

impl<'a, 'b> FrameworkCommandBuilder<'a, 'b> {
    /// Make a new [`FrameworkCommandBuilder`].
    pub fn new() -> Self {
        Self {
            name: None,
            description: None,
            on_process: None,
        }
    }

    /// The command name
    pub fn name(&mut self, name: &'a str) -> &mut Self {
        self.name = Some(name);
        self
    }

    /// The command description
    pub fn description(&mut self, description: &'b str) -> &mut Self {
        self.description = Some(description);
        self
    }

    /// The on_process hook
    pub fn on_process<P>(&mut self, on_process: P) -> &mut Self
    where
        P: Fn(Context, ApplicationCommandInteraction) -> OnProcessFuture + Send + Sync + 'static,
    {
        self.on_process = Some(Box::new(on_process));
        self
    }

    /// Build the [`FrameworkCommand`]
    pub fn build(&mut self) -> anyhow::Result<FrameworkCommand> {
        let name = self.name.take().context("missing name")?;
        let description = self.description.take().context("missing description")?;
        let on_process = self.on_process.take().context("missing on_process")?;

        Ok(FrameworkCommand {
            name: name.into(),
            description: description.into(),
            on_process,
        })
    }
}

impl std::fmt::Debug for FrameworkCommandBuilder<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameworkCommandBuilder")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("on_process", &"<func>")
            .finish()
    }
}

impl Default for FrameworkCommandBuilder<'_, '_> {
    fn default() -> Self {
        Self::new()
    }
}
