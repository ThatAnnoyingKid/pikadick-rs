mod argument;
mod check;
mod command;
mod convert;

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
    convert::{
        ConvertError,
        DataType,
        FromOptionValue,
    },
};
pub use crate::convert::FromOptions;
pub use pikadick_slash_framework_derive::FromOptions;
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
