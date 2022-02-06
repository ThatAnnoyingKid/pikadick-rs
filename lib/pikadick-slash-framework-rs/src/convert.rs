use serenity::model::prelude::application_command::{
    ApplicationCommandInteraction,
    ApplicationCommandInteractionDataOptionValue,
};

/// Error while converting from an interaction
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// The type is unknown
    #[error("unexpected type for '{name}', expected '{expected}', got '{actual:?}'")]
    UnexpectedType {
        /// Name of the field that failed.
        name: &'static str,
        /// The expected datatype
        expected: DataType,
        /// The actual datatype.
        ///
        /// This is `None` if the actual datatype is unknown.
        actual: Option<DataType>,
    },

    /// Missing a required field
    #[error("missing required field for '{name}', expected '{expected}'")]
    MissingRequiredField {
        /// the name of the missing field
        name: &'static str,
        /// The expected datatype
        expected: DataType,
    },
}

/// A trait that allows converting from an application command interaction
pub trait FromOptions: std::fmt::Debug + Send
where
    Self: Sized,
{
    /// Make arguments from a [`ApplicationCommandInteraction`]
    fn from_options(interaction: &ApplicationCommandInteraction) -> Result<Self, ConvertError>;
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
    /// This returns an option as [`DataType`] does not encode the concept of an unknown data type.
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

/// Convert from an option value
pub trait FromOptionValue: Sized {
    /// Parse from an option value
    fn from_option_value(
        name: &'static str,
        option: Option<&ApplicationCommandInteractionDataOptionValue>,
    ) -> Result<Self, ConvertError>;

    /// The expected data type
    fn get_expected_data_type() -> DataType;

    /// Kind of a hack to get the default "missing" value if the key was not present.
    ///
    /// # Returns
    /// Returns None if this type is not optional.
    fn get_missing_default() -> Option<Self> {
        None
    }
}

impl FromOptionValue for bool {
    fn from_option_value(
        name: &'static str,
        option: Option<&ApplicationCommandInteractionDataOptionValue>,
    ) -> Result<Self, ConvertError> {
        let expected = Self::get_expected_data_type();

        let option = match option {
            Some(option) => option,
            None => {
                return Err(ConvertError::MissingRequiredField { name, expected });
            }
        };

        match option {
            ApplicationCommandInteractionDataOptionValue::Boolean(b) => Ok(*b),
            t => Err(ConvertError::UnexpectedType {
                name,
                expected,
                actual: DataType::from_data_option_value(t),
            }),
        }
    }

    fn get_expected_data_type() -> DataType {
        DataType::Boolean
    }
}

impl<T> FromOptionValue for Option<T>
where
    T: FromOptionValue,
{
    fn from_option_value(
        name: &'static str,
        option: Option<&ApplicationCommandInteractionDataOptionValue>,
    ) -> Result<Self, ConvertError> {
        if option.is_none() {
            return Ok(None);
        }

        T::from_option_value(name, option).map(Some)
    }

    fn get_missing_default() -> Option<Self> {
        Some(None)
    }

    fn get_expected_data_type() -> DataType {
        T::get_expected_data_type()
    }
}
