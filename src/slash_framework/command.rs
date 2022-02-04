use super::OnProcessFuture;
use anyhow::Context as _;
use serenity::{
    model::prelude::application_command::ApplicationCommandInteraction,
    prelude::*,
};

/// A builder for a [`SlashFrameworkCommand`].
pub struct SlashFrameworkCommandBuilder<'a, 'b> {
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

impl<'a, 'b> SlashFrameworkCommandBuilder<'a, 'b> {
    /// Make a new [`SlashFrameworkCommandBuilder`].
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

    /// Build the [`SlashFrameworkCommand`]
    pub fn build(&mut self) -> anyhow::Result<SlashFrameworkCommand> {
        let name = self.name.take().context("missing name")?;
        let description = self.description.take().context("missing description")?;
        let on_process = self.on_process.take().context("missing on_process")?;

        Ok(SlashFrameworkCommand {
            name: name.into(),
            description: description.into(),
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

    /// Fire the on_process hook
    pub fn fire_on_process(
        &self,
        ctx: Context,
        interaction: ApplicationCommandInteraction,
    ) -> OnProcessFuture {
        (self.on_process)(ctx, interaction)
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
