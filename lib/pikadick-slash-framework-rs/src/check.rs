use crate::{
    BoxFuture,
    Command,
};
use serenity::{
    client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
};

pub type CheckFn = for<'a> fn(
    &'a Context,
    &'a ApplicationCommandInteraction,
    &'a Command,
) -> BoxFuture<'a, Result<(), Reason>>;

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
