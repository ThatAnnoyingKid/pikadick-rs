use anyhow::{
    ensure,
    Context as _,
};
pub use pikadick_slash_framework::{
    ArgumentKind as SlashFrameworkArgumentKind,
    ArgumentParamBuilder as SlashFrameworkArgumentBuilder,
    Command as SlashFrameworkCommand,
    CommandBuilder as SlashFrameworkCommandBuilder,
    ConvertError,
    FromApplicationCommandInteraction,
};
use pikadick_slash_framework::{
    BoxError,
    CheckFn,
};
use serenity::{
    model::{
        interactions::application_command::ApplicationCommand,
        prelude::{
            application_command::ApplicationCommandInteraction,
            *,
        },
    },
    prelude::*,
};
use std::{
    collections::HashMap,
    sync::Arc,
};
use tracing::{
    info,
    warn,
};

/// A wrapper for [`BoxError`] that impls error
struct WrapBoxError(BoxError);

impl WrapBoxError {
    /// Make a new [`WrapBoxError`] from an error
    fn new(e: BoxError) -> Self {
        Self(e)
    }
}

impl std::fmt::Debug for WrapBoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for WrapBoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for WrapBoxError {}

/// A framework
#[derive(Clone)]
pub struct SlashFramework {
    commands: Arc<HashMap<Box<str>, Arc<SlashFrameworkCommand>>>,
    checks: Arc<[CheckFn]>,
}

impl SlashFramework {
    /// Register the framework.
    ///
    /// `test_guild_id` is an optional guild where the commands will be registered as guild commands,
    /// so they update faster for testing purposes.
    pub async fn register(
        &self,
        ctx: Context,
        test_guild_id: Option<GuildId>,
    ) -> anyhow::Result<()> {
        for (_name, framework_command) in self.commands.iter() {
            ApplicationCommand::create_global_application_command(&ctx.http, |command| {
                framework_command.register(command);

                command
            })
            .await?;
        }

        if let Some(guild_id) = test_guild_id {
            GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
                for (_name, framework_command) in self.commands.iter() {
                    commands.create_application_command(|command| {
                        framework_command.register(command);
                        command
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
            self.process_interaction_create_application_command(ctx, command)
                .await
        }
    }

    #[tracing::instrument(skip(self, ctx, command), fields(id = %command.id, author = %command.user.id, guild = ?command.guild_id, channel_id = %command.channel_id))]
    async fn process_interaction_create_application_command(
        &self,
        ctx: Context,
        command: ApplicationCommandInteraction,
    ) {
        let framework_command = match self.commands.get(command.data.name.as_str()) {
            Some(command) => command,
            None => {
                // TODO: Fire unknown command
                return;
            }
        };

        // TODO: Consider making parallel
        let mut check_result = Ok(());
        for check in self.checks.iter() {
            check_result = check_result.and(check(&ctx, &command, framework_command).await);
        }

        match check_result {
            Ok(()) => {
                info!("processing command `{}`", framework_command.name());
                if let Err(e) = framework_command
                    .fire_on_process(ctx, command)
                    .await
                    .map_err(WrapBoxError::new)
                    .context("failed to process command")
                {
                    // TODO: handle error with handler
                    warn!("{:?}", e);
                }
            }
            Err(e) => {
                let content = if let Some(user) = e.user.as_deref() {
                    user
                } else {
                    "check failed for unknown reason"
                };

                if let Some(log) = e.log {
                    warn!("{}", log);
                }

                if let Err(e) = command
                    .create_interaction_response(&ctx.http, |res| {
                        res.interaction_response_data(|res| res.content(content))
                    })
                    .await
                    .context("failed to send check failure response")
                {
                    warn!("{:?}", e);
                }
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
    checks: Vec<CheckFn>,
}

impl SlashFrameworkBuilder {
    /// Make a new [`SlashFrameworkBuilder`].
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            checks: Vec::new(),
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

    /// Add a check
    pub fn check(&mut self, check: CheckFn) -> &mut Self {
        self.checks.push(check);
        self
    }

    /// Build a framework
    pub fn build(&mut self) -> anyhow::Result<SlashFramework> {
        Ok(SlashFramework {
            commands: Arc::new(std::mem::take(&mut self.commands)),
            checks: std::mem::take(&mut self.checks).into(),
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
