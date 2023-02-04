use crate::{
    ArgumentParam,
    BoxError,
    BoxFuture,
    BuilderError,
    CheckFn,
    ClientData,
    DataType,
    FromOptions,
};
use std::{
    collections::HashMap,
    future::Future,
    sync::Arc,
};
use twilight_model::application::{
    command::{
        BaseCommandOptionData as TwilightBaseOptionCommandData,
        ChoiceCommandOptionData as TwilightChoiceCommandOptionData,
        Command as TwilightCommand,
        CommandOption as TwilightCommandOption,
        CommandType as TwilightCommandType,
        NumberCommandOptionData as TwilightNumberCommandOptionData,
    },
    interaction::{
        application_command::CommandData,
        Interaction as TwilightInteraction,
    },
};
use twilight_util::builder::command::CommandBuilder as TwilightCommandBuilder;

type OnProcessResult = Result<(), BoxError>;
pub type OnProcessFuture = BoxFuture<'static, OnProcessResult>;

// Keep these types in sync.
type OnProcessFutureFn<D> =
    Box<dyn Fn(D, TwilightInteraction, Box<CommandData>) -> OnProcessFuture + Send + Sync>;
type OnProcessFutureFnPtr<D, F, A> = fn(D, TwilightInteraction, A) -> F;

type HelpOnProcessFutureFn<D> = Box<
    dyn Fn(
            D,
            TwilightInteraction,
            Box<CommandData>,
            Arc<HashMap<Box<str>, Command<D>>>,
        ) -> OnProcessFuture
        + Send
        + Sync,
>;
type HelpOnProcessFutureFnPtr<D, F, A> =
    fn(D, TwilightInteraction, Arc<HashMap<Box<str>, Command<D>>>, A) -> F;

/// A slash framework command
pub struct Command<D> {
    /// The name of the command
    name: Box<str>,

    /// Description
    description: Box<str>,

    /// Arguments
    arguments: Box<[ArgumentParam]>,

    /// The main "process" func
    on_process: OnProcessFutureFn<D>,

    /// Checks that must pass before this command is run
    checks: Vec<CheckFn<D>>,
}

impl<D> Command<D> {
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
        client_data: D,
        interaction: TwilightInteraction,
        command_data: Box<CommandData>,
    ) -> Result<(), BoxError> {
        (self.on_process)(client_data, interaction, command_data).await
    }

    /// Get the inner checks
    pub fn checks(&self) -> &[CheckFn<D>] {
        &self.checks
    }

    /// Build a twilight command from this command
    pub fn build_twilight_command(&self) -> TwilightCommand {
        let mut command = TwilightCommandBuilder::new(
            self.name(),
            self.description(),
            TwilightCommandType::ChatInput,
        );

        for argument in self.arguments().iter() {
            let option = match argument.kind() {
                DataType::Boolean => {
                    TwilightCommandOption::Boolean(TwilightBaseOptionCommandData {
                        description: argument.description().to_string(),
                        description_localizations: None,
                        name: argument.name().to_string(),
                        name_localizations: None,
                        required: argument.required(),
                    })
                }
                DataType::String => {
                    TwilightCommandOption::String(TwilightChoiceCommandOptionData {
                        autocomplete: false,
                        choices: Vec::new(),
                        description: argument.description().to_string(),
                        description_localizations: None,
                        max_length: None,
                        min_length: None,
                        name: argument.name().to_string(),
                        name_localizations: None,
                        required: argument.required(),
                    })
                }
                DataType::Integer => {
                    TwilightCommandOption::Integer(TwilightNumberCommandOptionData {
                        autocomplete: false,
                        choices: Vec::new(),
                        description: argument.description().to_string(),
                        description_localizations: None,
                        max_value: None,
                        min_value: None,
                        name: argument.name().to_string(),
                        name_localizations: None,
                        required: argument.required(),
                    })
                }
            };

            command = command.option(option);
        }

        command.build()
    }
}

impl<D> std::fmt::Debug for Command<D> {
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
pub struct CommandBuilder<'a, 'b, D> {
    name: Option<&'a str>,
    description: Option<&'b str>,
    arguments: Vec<ArgumentParam>,

    on_process: Option<OnProcessFutureFn<D>>,
    checks: Vec<CheckFn<D>>,
}

impl<'a, 'b, D> CommandBuilder<'a, 'b, D> {
    /// Make a new [`CommandBuilder`].
    pub fn new() -> Self {
        Self {
            name: None,
            description: None,
            arguments: Vec::new(),

            on_process: None,
            checks: Vec::new(),
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

    /// Add many arguments
    pub fn arguments(&mut self, arguments: impl Iterator<Item = ArgumentParam>) -> &mut Self {
        for argument in arguments {
            self.argument(argument);
        }
        self
    }
}

impl<'a, 'b, D> CommandBuilder<'a, 'b, D>
where
    D: ClientData,
{
    /// The on_process hook
    pub fn on_process<F, A>(&mut self, on_process: OnProcessFutureFnPtr<D, F, A>) -> &mut Self
    where
        F: Future<Output = Result<(), BoxError>> + Send + 'static,
        A: FromOptions + 'static,
    {
        // Trampoline so user does not have to box manually and parse their args manually
        self.on_process = Some(Box::new(move |client_data, interaction, command_data| {
            Box::pin(async move {
                let args = A::from_options(&command_data.options)?;
                (on_process)(client_data, interaction, args).await
            })
        }));

        self
    }

    /// Add a check to this specific command
    pub fn check(&mut self, check: CheckFn<D>) -> &mut Self {
        self.checks.push(check);
        self
    }

    /// Build the [`Command`]
    pub fn build(&mut self) -> Result<Command<D>, BuilderError> {
        let name = self.name.take().ok_or(BuilderError::MissingField("name"))?;
        let description = self
            .description
            .take()
            .ok_or(BuilderError::MissingField("description"))?;
        let on_process = self
            .on_process
            .take()
            .ok_or(BuilderError::MissingField("on_process"))?;
        let checks = std::mem::take(&mut self.checks);

        Ok(Command {
            name: name.into(),
            description: description.into(),
            arguments: std::mem::take(&mut self.arguments).into_boxed_slice(),

            on_process,
            checks,
        })
    }
}

impl<D> std::fmt::Debug for CommandBuilder<'_, '_, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandBuilder")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("arguments", &self.arguments)
            .field("on_process", &self.on_process.as_ref().map(|_| "<func>"))
            .finish()
    }
}

impl<D> Default for CommandBuilder<'_, '_, D> {
    fn default() -> Self {
        Self::new()
    }
}

/// A slash framework help command
pub struct HelpCommand<D> {
    /// Description
    description: Box<str>,

    /// Arguments
    arguments: Box<[ArgumentParam]>,

    /// The main "process" func
    on_process: HelpOnProcessFutureFn<D>,
}

impl<D> HelpCommand<D> {
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
        client_data: D,
        interaction: TwilightInteraction,
        command_data: Box<CommandData>,
        map: Arc<HashMap<Box<str>, Command<D>>>,
    ) -> Result<(), BoxError> {
        (self.on_process)(client_data, interaction, command_data, map).await
    }

    /// Build a twilight command from this command
    pub fn build_twilight_command(&self) -> TwilightCommand {
        let mut command =
            TwilightCommandBuilder::new("help", self.description(), TwilightCommandType::ChatInput);

        for argument in self.arguments().iter() {
            let option = match argument.kind() {
                DataType::Boolean => {
                    TwilightCommandOption::Boolean(TwilightBaseOptionCommandData {
                        description: argument.description().to_string(),
                        description_localizations: None,
                        name: argument.name().to_string(),
                        name_localizations: None,
                        required: argument.required(),
                    })
                }
                DataType::String => {
                    TwilightCommandOption::String(TwilightChoiceCommandOptionData {
                        autocomplete: false,
                        choices: Vec::new(),
                        description: argument.description().to_string(),
                        description_localizations: None,
                        max_length: None,
                        min_length: None,
                        name: argument.name().to_string(),
                        name_localizations: None,
                        required: argument.required(),
                    })
                }
                DataType::Integer => {
                    TwilightCommandOption::Integer(TwilightNumberCommandOptionData {
                        autocomplete: false,
                        choices: Vec::new(),
                        description: argument.description().to_string(),
                        description_localizations: None,
                        max_value: None,
                        min_value: None,
                        name: argument.name().to_string(),
                        name_localizations: None,
                        required: argument.required(),
                    })
                }
            };

            command = command.option(option);
        }

        command.build()
    }
}

impl<D> std::fmt::Debug for HelpCommand<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HelpCommand")
            .field("description", &self.description)
            .field("arguments", &self.arguments)
            .field("on_process", &"<func>")
            .finish()
    }
}

/// A builder for a [`HelpCommand`].
pub struct HelpCommandBuilder<'a, D> {
    description: Option<&'a str>,
    arguments: Vec<ArgumentParam>,

    on_process: Option<HelpOnProcessFutureFn<D>>,
}

impl<'a, D> HelpCommandBuilder<'a, D>
where
    D: ClientData,
{
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
    pub fn on_process<F, A>(&mut self, on_process: HelpOnProcessFutureFnPtr<D, F, A>) -> &mut Self
    where
        F: Future<Output = Result<(), BoxError>> + Send + 'static,
        A: FromOptions + 'static,
    {
        // Trampoline so user does not have to box manually and parse their args manually
        self.on_process = Some(Box::new(
            move |client_data, interaction, command_data, map| {
                Box::pin(async move {
                    let args = A::from_options(&command_data.options)?;
                    (on_process)(client_data, interaction, map, args).await
                })
            },
        ));

        self
    }

    /// Build the [`HelpCommand`]
    pub fn build(&mut self) -> Result<HelpCommand<D>, BuilderError> {
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

impl<D> std::fmt::Debug for HelpCommandBuilder<'_, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HelpCommandBuilder")
            .field("description", &self.description)
            .field("arguments", &self.arguments)
            .field("on_process", &self.on_process.as_ref().map(|_| "<func>"))
            .finish()
    }
}

impl<D> Default for HelpCommandBuilder<'_, D>
where
    D: ClientData,
{
    fn default() -> Self {
        Self::new()
    }
}
