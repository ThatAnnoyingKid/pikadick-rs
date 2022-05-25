use std::{
    fmt::{
        Debug,
        Display,
    },
    sync::Arc,
};

/// An arced anyhow error.
///
/// It is not intended as a replacement for anyhow's error,
/// just a way to share it as well as a way to provide an error impl.
#[derive(Clone)]
pub struct ArcAnyhowError(Arc<anyhow::Error>);

impl ArcAnyhowError {
    /// Make a new [`ArcAnyhowError`]
    pub fn new(error: anyhow::Error) -> Self {
        Self(Arc::new(error))
    }
}

impl Debug for ArcAnyhowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&*self.0, f)
    }
}

impl Display for ArcAnyhowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&*self.0, f)
    }
}

// We allow deprecated functions as this is just a wrapper,
// we want to emulate anyhow::error's choices,
// even if they use deprecated code
#[allow(deprecated)]
impl std::error::Error for ArcAnyhowError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.as_ref().source()
    }

    // TODO: Wait for backtraces to stabilize
    // fn backtrace(&self) -> Option<&Backtrace> {
    //     self.0.as_ref().backtrace()
    // }

    fn description(&self) -> &str {
        self.0.as_ref().description()
    }
    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.0.as_ref().cause()
    }
}
