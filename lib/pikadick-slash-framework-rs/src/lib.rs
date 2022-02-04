mod argument;
mod command;

pub use self::{
    argument::{
        ArgumentKind,
        ArgumentParam,
        ArgumentParamBuilder,
    },
    command::{
        Command,
        CommandBuilder,
        OnProcessFuture,
    },
};
use std::{
    future::Future,
    pin::Pin,
};

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A field is missing
    #[error("missing {0}")]
    MissingField(&'static str),
}
