use crate::{
    BoxFuture,
    Command,
};
use twilight_model::application::interaction::{
    application_command::CommandData,
    Interaction,
};

pub type CheckFn<D> = for<'a> fn(
    &'a D,
    &'a Interaction,
    &'a CommandData,
    &'a Command<D>,
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
