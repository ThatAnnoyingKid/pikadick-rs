#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A field is missing
    #[error("missing {0}")]
    MissingField(&'static str),
}

/// The kind of argument
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ArgumentKind {
    /// A boolean
    Boolean,
}

/// An argument.
///
/// Specifically, this is a parameter, not a value.
#[derive(Debug)]
pub struct ArgumentParam {
    name: Box<str>,
    kind: ArgumentKind,
    description: Box<str>,
}

impl ArgumentParam {
    /// Get the name of the argument
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the argument kind
    pub fn kind(&self) -> ArgumentKind {
        self.kind
    }

    /// Get the description of the argument
    pub fn description(&self) -> &str {
        &self.description
    }
}

/// An argument param builder
#[derive(Debug)]
pub struct ArgumentParamBuilder<'a, 'b> {
    name: Option<&'a str>,
    kind: Option<ArgumentKind>,
    description: Option<&'b str>,
}

impl<'a, 'b> ArgumentParamBuilder<'a, 'b> {
    /// Make a new [`ArgumentParamBuilder`].
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
    pub fn kind(&mut self, kind: ArgumentKind) -> &mut Self {
        self.kind = Some(kind);
        self
    }

    /// Set the description
    pub fn description(&mut self, description: &'b str) -> &mut Self {
        self.description = Some(description);
        self
    }

    /// Build the argument param
    pub fn build(&mut self) -> Result<ArgumentParam, Error> {
        let name = self.name.ok_or(Error::MissingField("name"))?;
        let kind = self.kind.ok_or(Error::MissingField("kind"))?;
        let description = self.description.ok_or(Error::MissingField("description"))?;

        Ok(ArgumentParam {
            name: name.into(),
            kind,
            description: description.into(),
        })
    }
}

impl<'a, 'b> Default for ArgumentParamBuilder<'a, 'b> {
    fn default() -> Self {
        Self::new()
    }
}
