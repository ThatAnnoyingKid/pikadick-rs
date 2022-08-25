use crate::{
    BuilderError,
    CheckFn,
    ClientData,
    Command,
    HelpCommand,
    WrapBoxError,
};
use std::{
    collections::HashMap,
    sync::Arc,
};
use tracing::{
    info,
    warn,
};
use twilight_http::client::InteractionClient;
use twilight_model::{
    application::interaction::{
        application_command::CommandData,
        Interaction as TwilightInteraction,
        InteractionData,
    },
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{
        InteractionResponse,
        InteractionResponseType,
    },
    id::{
        marker::{
            GuildMarker,
            UserMarker,
        },
        Id,
    },
};
use twilight_util::builder::InteractionResponseDataBuilder;

struct FmtOptionsHelper<'a>(&'a CommandData);

impl std::fmt::Display for FmtOptionsHelper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let len = self.0.options.len();
        for (i, option) in self.0.options.iter().enumerate() {
            if i + 1 == len {
                write!(f, "'{}'={:?}", option.name, option.value)?;
            }
        }
        write!(f, "]")?;

        Ok(())
    }
}

/// A framework
#[derive(Clone)]
pub struct Framework<D> {
    commands: Arc<HashMap<Box<str>, Command<D>>>,
    help_command: Option<Arc<HelpCommand<D>>>,
    checks: Arc<[CheckFn<D>]>,
}

impl<D> Framework<D>
where
    D: ClientData,
{
    /// Register the framework.
    ///
    /// `test_guild_id` is an optional guild where the commands will be registered as guild commands,
    /// so they update faster for testing purposes.
    pub async fn register(
        &self,
        interaction_client: InteractionClient<'_>,
        test_guild_id: Option<Id<GuildMarker>>,
    ) -> Result<(), twilight_http::Error> {
        let mut commands = Vec::with_capacity(self.commands.len());

        for twilight_command in self
            .commands
            .values()
            .map(|framework_command| framework_command.build_twilight_command())
            .chain(
                self.help_command
                    .as_deref()
                    .map(|framework_command| framework_command.build_twilight_command()),
            )
        {
            commands.push(twilight_command);
        }

        interaction_client
            .set_global_commands(&commands)
            .exec()
            .await?;

        if let Some(guild_id) = test_guild_id {
            interaction_client
                .set_guild_commands(guild_id, &commands)
                .exec()
                .await?;
        }

        Ok(())
    }

    /// Process an interaction create event
    pub async fn process_interaction_create(
        &self,
        client_data: D,
        mut interaction: Box<InteractionCreate>,
    ) {
        if let Some(InteractionData::ApplicationCommand(command)) = interaction.0.data.take() {
            // TODO: Can interaction.author_id ever return None?
            let author_id = interaction.author_id();
            self.process_interaction_create_application_command(
                client_data,
                interaction.0,
                author_id,
                command,
            )
            .await
        }
    }

    #[tracing::instrument(skip(self, client_data, command_data), fields(id = %command_data.id, author = ?author_id, guild = ?command_data.guild_id, channel_id = ?interaction.channel_id))]
    async fn process_interaction_create_application_command(
        &self,
        client_data: D,
        interaction: TwilightInteraction,
        author_id: Option<Id<UserMarker>>,
        command_data: Box<CommandData>,
    ) {
        if command_data.name.as_str() == "help" {
            // Keep comments
            #[allow(clippy::single_match)]
            match self.help_command.as_ref() {
                Some(framework_command) => {
                    info!(
                        "processing help command, options={}",
                        FmtOptionsHelper(&command_data)
                    );
                    if let Err(e) = framework_command
                        .fire_on_process(
                            client_data,
                            interaction,
                            command_data,
                            self.commands.clone(),
                        )
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

        let command_name = command_data.name.as_str();
        let framework_command = match self.commands.get(command_name) {
            Some(command) => command,
            None => {
                // TODO: Unknown command handler
                warn!("unknown command '{command_name}'");
                return;
            }
        };

        // TODO: Consider making parallel
        let mut check_result = Ok(());
        for check in self.checks.iter().chain(framework_command.checks().iter()) {
            check_result = check_result
                .and(check(&client_data, &interaction, &command_data, framework_command).await);
        }

        match check_result {
            Ok(()) => {
                info!(
                    "processing command `{}`, options={}",
                    framework_command.name(),
                    FmtOptionsHelper(&command_data)
                );
                if let Err(e) = framework_command
                    .fire_on_process(client_data, interaction, command_data)
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

                let response_data = InteractionResponseDataBuilder::new()
                    .content(content)
                    .build();
                let response = InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(response_data),
                };

                if let Err(e) = client_data
                    .interaction_client()
                    .create_response(interaction.id, &interaction.token, &response)
                    .exec()
                    .await
                {
                    warn!("{}", e);
                }
            }
        }
    }
}

impl<D> std::fmt::Debug for Framework<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Framework")
            .field("commands", &self.commands)
            .finish()
    }
}

/// A FrameworkBuilder for slash commands.
pub struct FrameworkBuilder<D> {
    commands: HashMap<Box<str>, Command<D>>,
    help_command: Option<HelpCommand<D>>,
    checks: Vec<CheckFn<D>>,

    error: Option<BuilderError>,
}

impl<D> FrameworkBuilder<D>
where
    D: ClientData,
{
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
    pub fn command(&mut self, command: Command<D>) -> &mut Self {
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
    pub fn help_command(&mut self, command: HelpCommand<D>) -> &mut Self {
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
    pub fn check(&mut self, check: CheckFn<D>) -> &mut Self {
        if self.error.is_some() {
            return self;
        }

        self.checks.push(check);
        self
    }

    /// Build a framework
    pub fn build(&mut self) -> Result<Framework<D>, BuilderError> {
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

impl<D> std::fmt::Debug for FrameworkBuilder<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameworkBuilder")
            .field("commands", &self.commands)
            .finish()
    }
}

impl<D> Default for FrameworkBuilder<D>
where
    D: ClientData,
{
    fn default() -> Self {
        Self::new()
    }
}
