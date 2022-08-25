mod argument;
mod check;
mod command;
mod convert;
mod framework;
mod util;

pub use self::{
    argument::{
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
        HelpCommand,
        HelpCommandBuilder,
        OnProcessFuture,
    },
    convert::{
        ConvertError,
        DataType,
        FromOptionValue,
    },
    framework::{
        Framework,
        FrameworkBuilder,
    },
};
pub use crate::convert::FromOptions;
pub(crate) use crate::util::{
    BoxError,
    WrapBoxError,
};
pub use pikadick_slash_framework_derive::FromOptions;
use std::{
    future::Future,
    pin::Pin,
};
use twilight_http::client::InteractionClient;

// Compat alias
// TODO: Deprecate
pub type ArgumentKind = DataType;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type BoxResult<T> = Result<T, BoxError>;

/// Builder Error
#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    /// A field is missing
    #[error("missing {0}")]
    MissingField(&'static str),

    /// Something was duplicated
    #[error("duplicate for key '{0}'")]
    Duplicate(Box<str>),
}

/// The client data for the framework
pub trait ClientData: Send + 'static {
    /// Get the interaction client
    fn interaction_client(&self) -> InteractionClient<'_>;
}
