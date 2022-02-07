use crate::{
    ArgumentKind,
    ArgumentParam,
    BoxError,
    BoxFuture,
    BuilderError,
    FromOptions,
};
use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::prelude::application_command::{
        ApplicationCommandInteraction,
        ApplicationCommandOptionType,
    },
};
use std::{
    collections::HashMap,
    future::Future,
    sync::Arc,
};

type OnProcessResult = Result<(), BoxError>;
pub type OnProcessFuture = BoxFuture<'static, OnProcessResult>;

// Keep these types in sync.
type OnProcessFutureFn =
    Box<dyn Fn(Context, ApplicationCommandInteraction) -> OnProcessFuture + Send + Sync>;
type OnProcessFutureFnPtr<F, A> = fn(Context, ApplicationCommandInteraction, A) -> F;

type HelpOnProcessFutureFn = Box<
    dyn Fn(
            Context,
            ApplicationCommandInteraction,
            Arc<HashMap<Box<str>, Command>>,
        ) -> OnProcessFuture
        + Send
        + Sync,
>;
type HelpOnProcessFutureFnPtr<F, A> =
    fn(Context, ApplicationCommandInteraction, Arc<HashMap<Box<str>, Command>>, A) -> F;

/// A slash framework command
pub struct Command {
    /// The name of the command
    name: Box<str>,

    /// Description
    description: Box<str>,

    /// Arguments
    arguments: Box<[ArgumentParam]>,

    /// The main "process" func
    on_process: OnProcessFutureFn,
}

impl Command {
    /// Get the command name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the command description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the command arguments
    pub fn arguments(&self) -> &[ArgumentParam] {
        &self.arguments
    }

    /// Fire the on_process hook
    pub async fn fire_on_process(
        &self,
        ctx: Context,
        interaction: ApplicationCommandInteraction,
    ) -> Result<(), BoxError> {
        (self.on_process)(ctx, interaction).await
    }

    /// Register this command
    pub fn register(&self, command: &mut CreateApplicationCommand) {
        command.name(self.name()).description(self.description());

        for argument in self.arguments().iter() {
            command.create_option(|option| {
                option
                    .name(argument.name())
                    .description(argument.description())
                    .kind(match argument.kind() {
                        ArgumentKind::Boolean => ApplicationCommandOptionType::Boolean,
                        ArgumentKind::String => ApplicationCommandOptionType::String,
                    })
                    .required(argument.required())
            });
        }
    }
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("arguments", &self.arguments)
            .field("on_process", &"<func>")
            .finish()
    }
}

/// A builder for a [`Command`].
pub struct CommandBuilder<'a, 'b> {
    name: Option<&'a str>,
    description: Option<&'b str>,
    arguments: Vec<ArgumentParam>,

    on_process: Option<OnProcessFutureFn>,
}

impl<'a, 'b> CommandBuilder<'a, 'b> {
    /// Make a new [`CommandBuilder`].
    pub fn new() -> Self {
        Self {
            name: None,
            description: None,
            arguments: Vec::new(),

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

    /// Add an argument
    pub fn argument(&mut self, argument: ArgumentParam) -> &mut Self {
        self.arguments.push(argument);
        self
    }

    /// The on_process hook
    pub fn on_process<F, A>(&mut self, on_process: OnProcessFutureFnPtr<F, A>) -> &mut Self
    where
        F: Future<Output = Result<(), BoxError>> + Send + 'static,
        A: FromOptions + 'static,
    {
        // Trampoline so user does not have to box manually and parse their args manually
        self.on_process = Some(Box::new(move |ctx, interaction| {
            Box::pin(async move {
                let args = A::from_options(&interaction)?;
                (on_process)(ctx, interaction, args).await
            })
        }));

        self
    }

    /// Build the [`Command`]
    pub fn build(&mut self) -> Result<Command, BuilderError> {
        let name = self.name.take().ok_or(BuilderError::MissingField("name"))?;
        let description = self
            .description
            .take()
            .ok_or(BuilderError::MissingField("description"))?;
        let on_process = self
            .on_process
            .take()
            .ok_or(BuilderError::MissingField("on_process"))?;

        Ok(Command {
            name: name.into(),
            description: description.into(),
            arguments: std::mem::take(&mut self.arguments).into_boxed_slice(),

            on_process,
        })
    }
}

impl std::fmt::Debug for CommandBuilder<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandBuilder")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("arguments", &self.arguments)
            .field("on_process", &self.on_process.as_ref().map(|_| "<func>"))
            .finish()
    }
}

impl Default for CommandBuilder<'_, '_> {
    fn default() -> Self {
        Self::new()
    }
}

/// A slash framework help command
pub struct HelpCommand {
    /// Description
    description: Box<str>,

    /// Arguments
    arguments: Box<[ArgumentParam]>,

    /// The main "process" func
    on_process: HelpOnProcessFutureFn,
}

impl HelpCommand {
    /// Get the help command description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the help command arguments
    pub fn arguments(&self) -> &[ArgumentParam] {
        &self.arguments
    }

    /// Fire the on_process hook
    pub async fn fire_on_process(
        &self,
        ctx: Context,
        interaction: ApplicationCommandInteraction,
        map: Arc<HashMap<Box<str>, Command>>,
    ) -> Result<(), BoxError> {
        (self.on_process)(ctx, interaction, map).await
    }

    /// Register this help command
    pub fn register(&self, command: &mut CreateApplicationCommand) {
        command.name("help").description(self.description());

        for argument in self.arguments().iter() {
            command.create_option(|option| {
                option
                    .name(argument.name())
                    .description(argument.description())
                    .kind(match argument.kind() {
                        ArgumentKind::Boolean => ApplicationCommandOptionType::Boolean,
                        ArgumentKind::String => ApplicationCommandOptionType::String,
                    })
            });
        }
    }
}

impl std::fmt::Debug for HelpCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HelpCommand")
            .field("description", &self.description)
            .field("arguments", &self.arguments)
            .field("on_process", &"<func>")
            .finish()
    }
}

/// A builder for a [`HelpCommand`].
pub struct HelpCommandBuilder<'a> {
    description: Option<&'a str>,
    arguments: Vec<ArgumentParam>,

    on_process: Option<HelpOnProcessFutureFn>,
}

impl<'a> HelpCommandBuilder<'a> {
    /// Make a new [`HelpCommandBuilder`].
    pub fn new() -> Self {
        Self {
            description: None,
            arguments: Vec::new(),

            on_process: None,
        }
    }

    /// The help command description
    pub fn description(&mut self, description: &'a str) -> &mut Self {
        self.description = Some(description);
        self
    }

    /// Add an argument
    pub fn argument(&mut self, argument: ArgumentParam) -> &mut Self {
        self.arguments.push(argument);
        self
    }

    /// The on_process hook
    pub fn on_process<F, A>(&mut self, on_process: HelpOnProcessFutureFnPtr<F, A>) -> &mut Self
    where
        F: Future<Output = Result<(), BoxError>> + Send + 'static,
        A: FromOptions + 'static,
    {
        // Trampoline so user does not have to box manually and parse their args manually
        self.on_process = Some(Box::new(move |ctx, interaction, map| {
            Box::pin(async move {
                let args = A::from_options(&interaction)?;
                (on_process)(ctx, interaction, map, args).await
            })
        }));

        self
    }

    /// Build the [`HelpCommand`]
    pub fn build(&mut self) -> Result<HelpCommand, BuilderError> {
        let description = self
            .description
            .take()
            .ok_or(BuilderError::MissingField("description"))?;
        let on_process = self
            .on_process
            .take()
            .ok_or(BuilderError::MissingField("on_process"))?;

        Ok(HelpCommand {
            description: description.into(),
            arguments: std::mem::take(&mut self.arguments).into_boxed_slice(),

            on_process,
        })
    }
}

impl std::fmt::Debug for HelpCommandBuilder<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HelpCommandBuilder")
            .field("description", &self.description)
            .field("arguments", &self.arguments)
            .field("on_process", &self.on_process.as_ref().map(|_| "<func>"))
            .finish()
    }
}

impl Default for HelpCommandBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}
