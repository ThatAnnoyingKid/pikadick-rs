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
use serenity::{
    client::Context,
    model::prelude::application_command::ApplicationCommandInteraction,
};
use std::{
    future::Future,
    pin::Pin,
};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
pub type BoxResult<T> = Result<T, BoxError>;

pub type CheckFn = for<'a> fn(
    &'a Context,
    &'a ApplicationCommandInteraction,
    &'a Command,
) -> BoxFuture<'a, Result<(), Reason>>;

/// Library Error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A field is missing
    #[error("missing {0}")]
    MissingField(&'static str),
}

/// the reason a check failed
pub struct Reason {
    /// The user-facing reason for a failure
    pub user: Option<String>,

    /// Info for logging
    pub log: Option<String>,
}

impl Reason {
    /// Create a new reason where nothing is known
    pub fn new_unknown() -> Self {
        Self {
            user: None,
            log: None,
        }
    }

    /// Create a new reason that has user data
    pub fn new_user<S>(user: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            user: Some(user.into()),
            log: None,
        }
    }
}
