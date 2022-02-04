use super::OnProcessFuture;
use anyhow::Context as _;
use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::application_command::{
        ApplicationCommandInteraction,
        ApplicationCommandOptionType,
    },
    prelude::*,
};

/// A builder for a [`SlashFrameworkCommand`].
pub struct SlashFrameworkCommandBuilder<'a, 'b> {
    name: Option<&'a str>,
    description: Option<&'b str>,
    arguments: Vec<SlashFrameworkArgument>,

    on_process: Option<
        Box<
            dyn Fn(Context, ApplicationCommandInteraction) -> OnProcessFuture
                + Send
                + Sync
                + 'static,
        >,
    >,
}

impl<'a, 'b> SlashFrameworkCommandBuilder<'a, 'b> {
    /// Make a new [`SlashFrameworkCommandBuilder`].
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
    pub fn argument(&mut self, argument: SlashFrameworkArgument) -> &mut Self {
        self.arguments.push(argument);
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

    /// Build the [`SlashFrameworkCommand`]
    pub fn build(&mut self) -> anyhow::Result<SlashFrameworkCommand> {
        let name = self.name.take().context("missing name")?;
        let description = self.description.take().context("missing description")?;
        let on_process = self.on_process.take().context("missing on_process")?;

        Ok(SlashFrameworkCommand {
            name: name.into(),
            description: description.into(),
            arguments: std::mem::take(&mut self.arguments),

            on_process,
        })
    }
}

impl std::fmt::Debug for SlashFrameworkCommandBuilder<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlashFrameworkCommandBuilder")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("on_process", &"<func>")
            .finish()
    }
}

impl Default for SlashFrameworkCommandBuilder<'_, '_> {
    fn default() -> Self {
        Self::new()
    }
}

/// A slash framework command
pub struct SlashFrameworkCommand {
    /// The name of the command
    name: Box<str>,

    /// Description
    description: Box<str>,

    /// Arguments
    arguments: Vec<SlashFrameworkArgument>,

    on_process: Box<
        dyn Fn(Context, ApplicationCommandInteraction) -> OnProcessFuture + Send + Sync + 'static,
    >,
}

impl SlashFrameworkCommand {
    /// Get the command name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the command description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the command arguments
    pub fn arguments(&self) -> &[SlashFrameworkArgument] {
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
    pub(super) fn register(&self, command: &mut CreateApplicationCommand) {
        command.name(self.name()).description(self.description());

        for argument in self.arguments().iter() {
            command.create_option(|option| {
                option
                    .name(&argument.name)
                    .description(&argument.description)
                    .kind(match argument.kind {
                        SlashFrameworkArgumentKind::Boolean => {
                            ApplicationCommandOptionType::Boolean
                        }
                    })
            });
        }
    }
}

impl std::fmt::Debug for SlashFrameworkCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlashFrameworkCommand")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("on_process", &"<func>")
            .finish()
    }
}

/// A slash framework argument builder
#[derive(Debug)]
pub struct SlashFrameworkArgumentBuilder<'a, 'b> {
    name: Option<&'a str>,
    kind: Option<SlashFrameworkArgumentKind>,
    description: Option<&'b str>,
}

impl<'a, 'b> SlashFrameworkArgumentBuilder<'a, 'b> {
    /// Make a new [`SlashFrameworkArgumentBuilder`].
    pub fn new() -> Self {
        Self {
            name: None,
            kind: None,
            description: None,
        }
    }

    /// Set the name
    pub fn name(&mut self, name: &'a str) -> &mut Self {
        self.name = Some(name);
        self
    }

    /// Set the kind
    pub fn kind(&mut self, kind: SlashFrameworkArgumentKind) -> &mut Self {
        self.kind = Some(kind);
        self
    }

    /// Set the description
    pub fn description(&mut self, description: &'b str) -> &mut Self {
        self.description = Some(description);
        self
    }

    /// Build the argument
    pub fn build(&mut self) -> anyhow::Result<SlashFrameworkArgument> {
        let name = self.name.context("missing name")?;
        let kind = self.kind.context("missing kind")?;
        let description = self.description.context("missing description")?;

        Ok(SlashFrameworkArgument {
            name: name.to_string(),
            kind,
            description: description.to_string(),
        })
    }
}

impl<'a, 'b> Default for SlashFrameworkArgumentBuilder<'a, 'b> {
    fn default() -> Self {
        Self::new()
    }
}

/// The kind of argument
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SlashFrameworkArgumentKind {
    /// A boolean
    Boolean,
}

/// An argument.
///
/// Specifically, this is a parameter, not a value.
#[derive(Debug)]
pub struct SlashFrameworkArgument {
    name: String,
    kind: SlashFrameworkArgumentKind,
    description: String,
}

/*
/// An argument for the slash framework
enum SlashFrameworkArgument {
    /// A boolean
    Boolean(bool),
}
*/
