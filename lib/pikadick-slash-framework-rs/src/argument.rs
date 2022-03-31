use crate::{
    BuilderError,
    DataType,
};

/// An argument.
///
/// Specifically, this is a parameter, not a value.
#[derive(Debug)]
pub struct ArgumentParam {
    name: Box<str>,
    kind: DataType,
    description: Box<str>,
    required: bool,
}

impl ArgumentParam {
    /// Get the name of the argument
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the argument kind
    pub fn kind(&self) -> DataType {
        self.kind
    }

    /// Get the description of the argument
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Check if the argument is required
    pub fn required(&self) -> bool {
        self.required
    }
}

/// An argument param builder
#[derive(Debug)]
pub struct ArgumentParamBuilder<'a, 'b> {
    name: Option<&'a str>,
    kind: Option<DataType>,
    description: Option<&'b str>,
    required: bool,
}

impl<'a, 'b> ArgumentParamBuilder<'a, 'b> {
    /// Make a new [`ArgumentParamBuilder`].
    pub fn new() -> Self {
        Self {
            name: None,
            kind: None,
            description: None,
            required: false,
        }
    }

    /// Set the name
    pub fn name(&mut self, name: &'a str) -> &mut Self {
        self.name = Some(name);
        self
    }

    /// Set the kind
    pub fn kind(&mut self, kind: DataType) -> &mut Self {
        self.kind = Some(kind);
        self
    }

    /// Set the description
    pub fn description(&mut self, description: &'b str) -> &mut Self {
        self.description = Some(description);
        self
    }

    /// Set if the argument is required
    pub fn required(&mut self, required: bool) -> &mut Self {
        self.required = required;
        self
    }

    /// Build the argument param
    pub fn build(&mut self) -> Result<ArgumentParam, BuilderError> {
        let name = self.name.ok_or(BuilderError::MissingField("name"))?;
        let kind = self.kind.ok_or(BuilderError::MissingField("kind"))?;
        let description = self
            .description
            .ok_or(BuilderError::MissingField("description"))?;

        Ok(ArgumentParam {
            name: name.into(),
            kind,
            description: description.into(),
            required: self.required,
        })
    }
}

impl<'a, 'b> Default for ArgumentParamBuilder<'a, 'b> {
    fn default() -> Self {
        Self::new()
    }
}
