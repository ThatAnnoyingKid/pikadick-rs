use crate::{
    BoxError,
    BuilderError,
    CheckFn,
    Command,
    HelpCommand,
};
use serenity::{
    client::Context,
    model::{
        application::{
            command::Command as ApplicationCommand,
            interaction::{
                application_command::ApplicationCommandInteraction,
                Interaction,
            },
        },
        prelude::GuildId,
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
    commands: Arc<HashMap<Box<str>, Command>>,
    help_command: Option<Arc<HelpCommand>>,
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
        for framework_command in self.commands.values() {
            ApplicationCommand::create_global_application_command(&ctx.http, |command| {
                framework_command.register(command);

                command
            })
            .await?;
        }

        if let Some(framework_command) = self.help_command.as_deref() {
            ApplicationCommand::create_global_application_command(&ctx.http, |command| {
                framework_command.register(command);

                command
            })
            .await?;
        }

        if let Some(guild_id) = test_guild_id {
            GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
                for framework_command in self.commands.values() {
                    commands.create_application_command(|command| {
                        framework_command.register(command);
                        command
                    });
                }

                if let Some(framework_command) = self.help_command.as_deref() {
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
        if command.data.name.as_str() == "help" {
            // Keep comments
            #[allow(clippy::single_match)]
            match self.help_command.as_ref() {
                Some(framework_command) => {
                    info!(
                        "processing help command, options={}",
                        FmtOptionsHelper(&command)
                    );
                    if let Err(e) = framework_command
                        .fire_on_process(ctx, command, self.commands.clone())
                        .await
                        .map_err(WrapBoxError::new)
                    {
                        // TODO: handle error with handler
                        warn!("{}", e);
                    }
                }
                None => {
                    // Don't log, as we assume the user does not want to provide help.
                    // Logging would be extra noise.
                }
            }

            return;
        }

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
        for check in self.checks.iter().chain(framework_command.checks().iter()) {
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
    commands: HashMap<Box<str>, Command>,
    help_command: Option<HelpCommand>,
    checks: Vec<CheckFn>,

    error: Option<BuilderError>,
}

impl FrameworkBuilder {
    /// Make a new [`FrameworkBuilder`].
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            help_command: None,
            checks: Vec::new(),

            error: None,
        }
    }

    /// Add a command
    pub fn command(&mut self, command: Command) -> &mut Self {
        if self.error.is_some() {
            return self;
        }

        let command_name: Box<str> = command.name().into();

        // A help command cannot be registered like this
        if &*command_name == "help" {
            self.error = Some(BuilderError::Duplicate(command_name));
            return self;
        }

        // Don't overwrite commands
        if self.commands.get(&command_name).is_some() {
            self.error = Some(BuilderError::Duplicate(command_name));
            return self;
        }

        self.commands.insert(command_name, command);

        self
    }

    /// Add a help command
    pub fn help_command(&mut self, command: HelpCommand) -> &mut Self {
        if self.error.is_some() {
            return self;
        }

        // Don't overwrite commands
        if self.help_command.is_some() {
            self.error = Some(BuilderError::Duplicate("help".into()));
            return self;
        }

        self.help_command = Some(command);

        self
    }

    /// Add a check
    pub fn check(&mut self, check: CheckFn) -> &mut Self {
        if self.error.is_some() {
            return self;
        }

        self.checks.push(check);
        self
    }

    /// Build a framework
    pub fn build(&mut self) -> Result<Framework, BuilderError> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }

        Ok(Framework {
            commands: Arc::new(std::mem::take(&mut self.commands)),
            help_command: self.help_command.take().map(Arc::new),

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
