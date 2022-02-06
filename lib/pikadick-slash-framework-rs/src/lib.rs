mod argument;
mod check;
mod command;

pub use self::{
    argument::{
        ArgumentKind,
        ArgumentParam,
        ArgumentParamBuilder,
    },
    check::{
        CheckFn,
        Reason,
    },
    command::{
        Command,
        CommandBuilder,
        OnProcessFuture,
    },
};
use serenity::model::prelude::application_command::{
    ApplicationCommandInteraction,
    ApplicationCommandInteractionDataOptionValue,
};
use std::{
    future::Future,
    pin::Pin,
};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
pub type BoxResult<T> = Result<T, BoxError>;

/// Library Error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A field is missing
    #[error("missing {0}")]
    MissingField(&'static str),
}

/// A trait that allows converting from an application command interaction
pub trait FromApplicationCommandInteraction: std::fmt::Debug + Send
where
    Self: Sized,
{
    fn from_interaction(interaction: &ApplicationCommandInteraction) -> Result<Self, ConvertError>;
}

/// Error while converting from an interaction
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// The type is unknown
    #[error("unexpected type for '{name}', expected '{expected}', got ''")]
    UnexpectedType {
        /// Name of the field that failed
        name: &'static str,
        /// The expected datatype
        expected: DataType,
        /// The actual datatype.
        ///
        /// This is `None` if the actual datatype is unknown.
        actual: Option<DataType>,
    },
}

/// A datatype
#[derive(Debug, Copy, Clone)]
pub enum DataType {
    /// A string
    String,

    /// Integer
    Integer,

    /// Bool
    Boolean,
}

impl DataType {
    /// Get this as a str
    pub fn as_str(self) -> &'static str {
        match self {
            Self::String => "String",
            Self::Integer => "i64",
            Self::Boolean => "bool",
        }
    }

    /// Get the type of a [`ApplicationCommandInteractionDataOptionValue`].
    ///
    /// This returns an option as [`DataType`] does not encode the concept of an unknown data type
    pub fn from_data_option_value(
        v: &ApplicationCommandInteractionDataOptionValue,
    ) -> Option<Self> {
        match v {
            ApplicationCommandInteractionDataOptionValue::String(_) => Some(DataType::String),
            ApplicationCommandInteractionDataOptionValue::Integer(_) => Some(DataType::Integer),
            ApplicationCommandInteractionDataOptionValue::Boolean(_) => Some(DataType::Boolean),
            _ => None,
        }
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
