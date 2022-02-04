use crate::{
    ArgumentKind,
    ArgumentParam,
    BoxError,
    BoxFuture,
    Error,
};
use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::prelude::application_command::{
        ApplicationCommandInteraction,
        ApplicationCommandOptionType,
    },
};
use std::future::Future;

pub type OnProcessFuture = BoxFuture<Result<(), BoxError>>;
type OnProcessFutureFn =
    Box<dyn Fn(Context, ApplicationCommandInteraction) -> OnProcessFuture + Send + Sync + 'static>;

/// A slash framework command
pub struct Command {
    /// The name of the command
    name: Box<str>,

    /// Description
    description: Box<str>,

    /// Arguments
    arguments: Vec<ArgumentParam>,

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
    pub fn fire_on_process(
        &self,
        ctx: Context,
        interaction: ApplicationCommandInteraction,
    ) -> OnProcessFuture {
        (self.on_process)(ctx, interaction)
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
                    })
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
    pub fn on_process<P, F>(&mut self, on_process: P) -> &mut Self
    where
        P: Fn(Context, ApplicationCommandInteraction) -> F + Send + Sync + 'static,
        F: Future<Output = Result<(), BoxError>> + Send + 'static,
    {
        // Trampoline so user does not have to box manually
        self.on_process = Some(Box::new(move |ctx, interaction| {
            Box::pin(on_process(ctx, interaction))
        }));

        self
    }

    /// Build the [`Command`]
    pub fn build(&mut self) -> Result<Command, Error> {
        let name = self.name.take().ok_or(Error::MissingField("name"))?;
        let description = self
            .description
            .take()
            .ok_or(Error::MissingField("description"))?;
        let on_process = self
            .on_process
            .take()
            .ok_or(Error::MissingField("on_process"))?;

        Ok(Command {
            name: name.into(),
            description: description.into(),
            arguments: std::mem::take(&mut self.arguments),

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
            .field("on_process", &"<func>")
            .finish()
    }
}

impl Default for CommandBuilder<'_, '_> {
    fn default() -> Self {
        Self::new()
    }
}
