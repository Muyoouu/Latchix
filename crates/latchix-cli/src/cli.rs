use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::logging::LogFormat;

/// API-first SFTP server with tenant-scoped control.
#[derive(Debug, Parser)]
#[command(name = "latchix", version, about, long_about = None)]
pub struct Cli {
    /// Path to the TOML configuration file.
    #[arg(
        long,
        global = true,
        env = "LATCHIX_CONFIG",
        default_value = "/etc/latchix/config.toml",
        value_name = "PATH"
    )]
    pub config: PathBuf,

    /// Log output format.
    #[arg(long, global = true, value_enum, default_value_t = LogFormat::Text)]
    pub log_format: LogFormat,

    /// Suppress informational logs.
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// Available commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start the HTTP health listener and the SSH/SFTP listener.
    Serve(ServeArgs),
    /// Apply embedded database migrations.
    Migrate,
    /// Write a consistent metadata and storage backup.
    Backup(BackupArgs),
    /// Check configuration and runtime paths.
    Health,
    /// Print a one-time bootstrap token.
    BootstrapToken(BootstrapTokenArgs),
    /// Diagnose configuration, permissions, and build features.
    Doctor,
}

/// Arguments for `serve`.
#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Seconds to wait for active tasks during shutdown.
    #[arg(long, default_value_t = 30, value_name = "SECONDS")]
    pub shutdown_grace_seconds: u64,
}

/// Arguments for `backup`.
#[derive(Debug, Args)]
pub struct BackupArgs {
    /// Destination path for the backup archive.
    #[arg(long, value_name = "PATH")]
    pub output: PathBuf,
}

/// Arguments for `bootstrap-token`.
#[derive(Debug, Args)]
pub struct BootstrapTokenArgs {
    /// Token lifetime in seconds.
    #[arg(long, default_value_t = 3600, value_name = "SECONDS")]
    pub ttl_seconds: u64,

    /// Emit machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}
