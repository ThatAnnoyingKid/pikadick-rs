use crate::{
    BoxError,
    BuilderError,
    CheckFn,
    Command,
};
use serenity::{
    client::Context,
    model::prelude::{
        application_command::{
            ApplicationCommand,
            ApplicationCommandInteraction,
        },
        GuildId,
        Interaction,
    },
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

struct FmtOptionsHelper<'a>(&'a ApplicationCommandInteraction);

impl std::fmt::Display for FmtOptionsHelper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let len = self.0.data.options.len();
        for (i, option) in self.0.data.options.iter().enumerate() {
            if i + 1 == len {
                write!(f, "'{}'={:?}", option.name, option.resolved)?;
            }
        }
        write!(f, "]")?;

        Ok(())
    }
}

/// A framework
#[derive(Clone)]
pub struct Framework {
    commands: Arc<HashMap<Box<str>, Arc<Command>>>,
    checks: Arc<[CheckFn]>,
}

impl Framework {
    /// Register the framework.
    ///
    /// `test_guild_id` is an optional guild where the commands will be registered as guild commands,
    /// so they update faster for testing purposes.
    pub async fn register(
        &self,
        ctx: Context,
        test_guild_id: Option<GuildId>,
    ) -> Result<(), serenity::Error> {
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
                // TODO: Unknown command handler
                warn!("unknown command '{}'", command.data.name.as_str());
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
                info!(
                    "processing command `{}`, options={}",
                    framework_command.name(),
                    FmtOptionsHelper(&command)
                );
                if let Err(e) = framework_command
                    .fire_on_process(ctx, command)
                    .await
                    .map_err(WrapBoxError::new)
                {
                    // TODO: handle error with handler
                    warn!("{}", e);
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
                {
                    warn!("{}", e);
                }
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
    commands: HashMap<Box<str>, Arc<Command>>,
    checks: Vec<CheckFn>,
}

impl FrameworkBuilder {
    /// Make a new [`FrameworkBuilder`].
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            checks: Vec::new(),
        }
    }

    /// Add a command
    pub fn command(&mut self, command: Command) -> Result<&mut Self, BuilderError> {
        let command_name: Box<str> = command.name().into();
        let had_duplicate = self
            .commands
            .insert(command_name.clone(), Arc::new(command))
            .is_some();

        if had_duplicate {
            return Err(BuilderError::Duplicate(command_name));
        }

        Ok(self)
    }

    /// Add a check
    pub fn check(&mut self, check: CheckFn) -> &mut Self {
        self.checks.push(check);
        self
    }

    /// Build a framework
    pub fn build(&mut self) -> Result<Framework, BuilderError> {
        Ok(Framework {
            commands: Arc::new(std::mem::take(&mut self.commands)),
            checks: std::mem::take(&mut self.checks).into(),
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
