use crate::config::ConfigError;

/// Errors that end a command.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Configuration could not be loaded or validated.
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),

    /// A health check failed.
    #[error("health check failed: {0}")]
    Health(String),

    /// Doctor found a problem.
    #[error("doctor found a problem: {0}")]
    Doctor(String),

    /// The command is not implemented in this build.
    #[error("{0} is not implemented in this build")]
    NotImplemented(&'static str),

    /// The server failed to start or stop.
    #[error("server error: {0}")]
    Serve(String),

    /// An I/O operation failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl AppError {
    /// Process exit code for this error.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Config(_) => 4,
            Self::Health(_) => 5,
            Self::Doctor(_) | Self::Serve(_) | Self::Io(_) => 1,
            Self::NotImplemented(_) => 3,
        }
    }
}
