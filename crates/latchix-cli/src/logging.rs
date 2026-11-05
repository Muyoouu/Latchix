use std::io::IsTerminal;

use clap::ValueEnum;
use tracing_subscriber::EnvFilter;

/// Log output format.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum LogFormat {
    /// Human-readable text.
    Text,
    /// One JSON object per line.
    Json,
}

/// Initializes tracing once. Initialization failure is ignored so a second
/// call in a test binary does not panic.
pub fn init(format: LogFormat, quiet: bool) {
    let default_filter = if quiet { "error" } else { "info" };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal());

    match format {
        LogFormat::Text => {
            let _ = builder.try_init();
        }
        LogFormat::Json => {
            let _ = builder.json().try_init();
        }
    }
}
